# ------------------------------------------------------------------------------------------------
# server.py
# ------------------------------------------------------------------------------------------------
from http.server import SimpleHTTPRequestHandler, HTTPServer
import mimetypes
import os

class CORSHTTPRequestHandler(SimpleHTTPRequestHandler):

    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()

    def guess_type(self, path):
        mimetype = mimetypes.guess_type(path)[0]
        if mimetype is None:
            mimetype = 'application/octet-stream'
        if path.endswith('.js'):
            mimetype = 'application/javascript'
        elif path.endswith('.wasm'):
            mimetype = 'application/wasm'
        return mimetype

def run(port=8000):
    server_address = ('', port)
    httpd = HTTPServer(server_address, CORSHTTPRequestHandler)
    print(f"Server running on port {port}")
    httpd.serve_forever()

if __name__ == '__main__':
    run()

