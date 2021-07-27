import falcon

from email.utils import formatdate


class PlaintextResource(object):
    def on_get(self, request, response):
        response.set_header(
            "Date", formatdate(timeval=None, localtime=False, usegmt=True)
        )
        response.set_header("Content-Type", "text/plain")
        response.body = b"Hello, world!"


print(falcon)

# app = falcon.App()
# app.add_route("/plaintext", PlaintextResource())


# if __name__ == "__main__":
#     from wsgiref import simple_server

#     httpd = simple_server.make_server("localhost", 8080, app)
#     httpd.serve_forever()
