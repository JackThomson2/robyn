# default imports
import os
import argparse
import asyncio
import inspect

from .robyn import Server, SocketHeld, prepare_to_run
from .responses import static_file, jsonify, text
from .dev_event_handler import EventHandler
from .log_colors import Colors

from multiprocessing import Process
from watchdog.observers import Observer


def spawned_process(handlers, socket, name):
    import asyncio
    import uvloop

    uvloop.install()
    loop = uvloop.new_event_loop()
    asyncio.set_event_loop(loop)

    # create a server
    server = Server()
    prepare_to_run()

    for i in handlers:
        server.add_route(i[0], i[1], i[2], i[3])

    server.start(socket, name)
    asyncio.get_event_loop().run_forever()


class Robyn:
    """This is the python wrapper for the Robyn binaries."""

    def __init__(self, file_object):
        directory_path = os.path.dirname(os.path.abspath(file_object))
        self.file_path = file_object
        self.directory_path = directory_path
        self.dev = self._is_dev()
        self.routes = []
        self.headers = []

    def _is_dev(self):
        parser = argparse.ArgumentParser()
        parser.add_argument(
            "--dev", default=False, type=lambda x: (str(x).lower() == "true")
        )
        return parser.parse_args().dev

    def add_route(self, route_type, endpoint, handler):
        """
        [This is base handler for all the decorators]

        :param route_type [str]: [route type between GET/POST/PUT/DELETE/PATCH]
        :param endpoint [str]: [endpoint for the route added]
        :param handler [function]: [represents the sync or async function passed as a handler for the route]
        """

        """ We will add the status code here only
        """
        self.routes.append(
            (route_type, endpoint, handler, asyncio.iscoroutinefunction(handler))
        )

    def add_header(self, key, value):
        self.headers.append((key, value))

    def remove_header(self, key):
        self.headers.remove(key)

    def start(self, port):
        """
        [Starts the server]

        :param port [int]: [reperesents the port number at which the server is listening]
        """

        socket = SocketHeld(f"0.0.0.0:{port}", port)

        if not self.dev:
            for i in range(2):
                copied = socket.try_clone()
                p = Process(
                    target=spawned_process,
                    args=(self.routes, copied, f"Process {i}"),
                )
                p.start()

            input("Press Cntrl + C to stop \n")
        else:
            event_handler = EventHandler(self.file_path)
            event_handler.start_server_first_time()
            print(
                f"{Colors.OKBLUE}Dev server initialised with the directory_path : {self.directory_path}{Colors.ENDC}"
            )
            observer = Observer()
            observer.schedule(event_handler, path=self.directory_path, recursive=True)
            observer.start()
            try:
                while True:
                    pass
            finally:
                observer.stop()
                observer.join()

    def get(self, endpoint):
        """
        [The @app.get decorator to add a get route]

        :param endpoint [str]: [endpoint to server the route]
        """

        def inner(handler):
            self.add_route("GET", endpoint, handler)

        return inner

    def post(self, endpoint):
        """
        [The @app.post decorator to add a get route]

        :param endpoint [str]: [endpoint to server the route]
        """

        def inner(handler):
            sig = inspect.signature(handler)
            params = len(sig.parameters)
            if params != 1:
                print("We need one argument on post.")
                return
            self.add_route("POST", endpoint, handler)

        return inner

    def put(self, endpoint):
        """
        [The @app.put decorator to add a get route]

        :param endpoint [str]: [endpoint to server the route]
        """

        def inner(handler):
            self.add_route("PUT", endpoint, handler)

        return inner

    def delete(self, endpoint):
        """
        [The @app.delete decorator to add a get route]

        :param endpoint [str]: [endpoint to server the route]
        """

        def inner(handler):
            self.add_route("DELETE", endpoint, handler)

        return inner

    def patch(self, endpoint):
        """
        [The @app.patch decorator to add a get route]

        :param endpoint [str]: [endpoint to server the route]
        """

        def inner(handler):
            self.add_route("PATCH", endpoint, handler)

        return inner
