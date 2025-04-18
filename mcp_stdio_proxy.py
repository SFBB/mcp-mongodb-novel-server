import sys
import json
import requests
import time

MCP_SERVER_URL = "http://localhost:3000/mcp"
LOG_PATH = "/home/tomtony/Documents/codes/rust/mcp_database/mcp_proxy.log"

def read_message():
    # Try to read a line (newline-delimited JSON, as sent by VS Code)
    line = sys.stdin.buffer.readline()
    if not line:
        return None
    line = line.strip()
    if not line:
        return None
    try:
        return json.loads(line.decode())
    except Exception:
        # Fallback: try to read Content-Length style (LSP)
        header = line
        while not header.endswith(b"\r\n\r\n"):
            chunk = sys.stdin.buffer.read(1)
            if not chunk:
                return None
            header += chunk
        headers = header.decode().split("\r\n")
        content_length = 0
        for h in headers:
            if h.lower().startswith("content-length:"):
                content_length = int(h.split(":")[1].strip())
        content = sys.stdin.buffer.read(content_length)
        if not content:
            return None
        return json.loads(content.decode())

def send_message(message):
    content = json.dumps(message, separators=(',', ':')).encode('utf-8')
    full_msg = b"" + content + b"\n"
    with open("/home/tomtony/Documents/codes/rust/mcp_database/mcp_proxy.log", "a") as log_file:
        print("[proxy] sending bytes:", full_msg, file=log_file, flush=True)
    sys.stdout.buffer.write(full_msg)
    sys.stdout.buffer.flush()

def main():
    with open(LOG_PATH, "a") as log_file:
        while True:
            try:
                request = read_message()
                if request is None:
                    print("[proxy] stdin closed, exiting.", file=log_file, flush=True)
                    break
                print("[proxy] received request:", request, file=log_file, flush=True)
                if request.get("method") == "exit":
                    print("[proxy] received exit request, exiting.", file=log_file, flush=True)
                    break
                resp = requests.post(MCP_SERVER_URL, json=request)
                print(resp, file=log_file, flush=True)
                response = resp.json()
                print("[proxy] received response:", response, file=log_file, flush=True)
                send_message(response)
            except Exception as e:
                print("[proxy] error:", e, file=log_file, flush=True)
                error_response = {
                    "jsonrpc": "2.0",
                    "id": request.get("id") if 'request' in locals() and request else None,
                    "error": {"code": -32000, "message": str(e)}
                }
                send_message(error_response)
            time.sleep(2)

if __name__ == "__main__":
    main()
