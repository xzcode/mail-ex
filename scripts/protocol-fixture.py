#!/usr/bin/env python3
"""Loopback-only IMAP/SMTP fixture, with a temporary trusted CA. Linux/OpenSSL."""
import argparse
import base64
import os
from pathlib import Path
import re
import shlex
import socketserver
import ssl
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[1]
RAW = (b"From: Sender <sender@example.invalid>\r\n"
       b"To: fixture@example.invalid, other@example.invalid\r\n"
       b"Subject: Fixture message\r\n"
       b"Date: Fri, 27 Nov 2026 00:00:00 +0000\r\n"
       b"Message-ID: <fixture-1@example.invalid>\r\n"
       b"Content-Type: text/plain; charset=utf-8\r\n\r\nFixture body.\r\n")
MAILBOXES = {"INBOX": {1: {"raw": RAW, "flags": set()}},
             "Sent": {}, "Archive": {}, "Trash": {}}
ROLES = {"INBOX": "\\HasNoChildren", "Sent": "\\Sent",
         "Archive": "\\Archive", "Trash": "\\Trash"}
DELIVERIES = []
LOCK = threading.Lock()

class TLSFixture(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True
    def __init__(self, handler, context):
        self.context = context
        super().__init__(("127.0.0.1", 0), handler)
    def get_request(self):
        conn, addr = super().get_request()
        conn.settimeout(10)
        try:
            return self.context.wrap_socket(conn, server_side=True), addr
        except Exception:
            conn.close()
            raise

class Handler(socketserver.StreamRequestHandler):
    def line(self, value):
        if isinstance(value, str):
            value = value.encode("ascii")
        self.wfile.write(value + b"\r\n")
        self.wfile.flush()

class IMAP(Handler):
    def handle(self):
        self.line("* OK Mail EX local IMAP fixture")
        selected = "INBOX"
        while True:
            raw = self.rfile.readline()
            if not raw:
                return
            parts = shlex.split(raw.decode("ascii").strip())
            if len(parts) < 2:
                return
            tag, command, *args = parts
            cmd = command.upper()
            ok = True
            with LOCK:
                if cmd == "CAPABILITY":
                    self.line("* CAPABILITY IMAP4rev1 MOVE UIDPLUS")
                elif cmd == "LOGIN":
                    ok = args == ["fixture@example.invalid", "fixture-only-secret"]
                elif cmd == "LIST":
                    for name, role in ROLES.items():
                        self.line(f'* LIST ({role}) "/" "{name}"')
                elif cmd in ("SELECT", "EXAMINE"):
                    selected = args[0]
                    if selected not in MAILBOXES:
                        ok = False
                    else:
                        self.line("* FLAGS (\\Seen \\Flagged \\Deleted)")
                        self.line(f"* {len(MAILBOXES[selected])} EXISTS")
                        self.line("* 0 RECENT")
                        self.line("* OK [UIDVALIDITY 100] Stable fixture")
                        self.line("* OK [UIDNEXT 20] Next UID")
                        self.line(f"{tag} OK [{'READ-ONLY' if cmd == 'EXAMINE' else 'READ-WRITE'}] Selected")
                        continue
                elif cmd == "UID" and args:
                    op = args[0].upper()
                    box = MAILBOXES[selected]
                    if op == "SEARCH":
                        cond = " ".join(args[1:]).upper()
                        ids = list(box)
                        if "UNSEEN" in cond:
                            ids = [i for i in ids if "\\Seen" not in box[i]["flags"]]
                        if "FLAGGED" in cond:
                            ids = [i for i in ids if "\\Flagged" in box[i]["flags"]]
                        if cond.startswith("UID "):
                            ids = [i for i in ids if i == int(cond.split()[1])]
                        self.line("* SEARCH" + (" " + " ".join(map(str, ids)) if ids else ""))
                    elif op == "FETCH":
                        for uid in map(int, args[1].split(",")):
                            if uid not in box:
                                continue
                            msg = box[uid]["raw"]
                            seq = list(box).index(uid) + 1
                            self.line(f"* {seq} FETCH (UID {uid} BODY[] {{{len(msg)}}}")
                            self.wfile.write(msg + b")\r\n")
                            self.wfile.flush()
                    elif op == "STORE":
                        uid = int(args[1])
                        if uid not in box:
                            ok = False
                        else:
                            operation = args[2]
                            # Use the original command, not shlex, to retain the flag's backslash.
                            match = re.search(rb"\((\\Seen|\\Flagged)\)", raw, re.I)
                            if not match:
                                ok = False
                            else:
                                flag = "\\Seen" if match.group(1).lower() == b"\\seen" else "\\Flagged"
                                flags = box[uid]["flags"]
                                if operation.startswith("+"):
                                    flags.add(flag)
                                else:
                                    flags.discard(flag)
                                self.line(f"* {list(box).index(uid)+1} FETCH (UID {uid} FLAGS ({' '.join(sorted(flags))}))")
                    elif op == "MOVE":
                        uid = int(args[1])
                        dest = args[2]
                        if uid not in box or dest not in MAILBOXES:
                            ok = False
                        else:
                            seq = list(box).index(uid)+1
                            MAILBOXES[dest][10] = box.pop(uid)
                            self.line(f"* {seq} EXPUNGE")
                    else:
                        ok = False
                elif cmd == "LOGOUT":
                    self.line("* BYE Local fixture")
                    self.line(f"{tag} OK Logout")
                    return
                elif cmd != "NOOP":
                    ok = False
            self.line(f"{tag} {'OK' if ok else 'NO'} Fixture response")

class SMTP(Handler):
    def handle(self):
        self.line("220 localhost Mail EX local SMTP fixture")
        authenticated = False
        recipients = []
        while True:
            raw = self.rfile.readline()
            if not raw:
                return
            text = raw.decode("ascii", errors="replace").strip()
            command, _, value = text.partition(" ")
            command = command.upper()
            if command in ("EHLO", "HELO"):
                self.line("250-localhost")
                self.line("250-AUTH PLAIN")
                self.line("250 8BITMIME")
            elif command == "AUTH" and value.startswith("PLAIN"):
                _, _, token = value.partition(" ")
                if not token:
                    self.line("334 ")
                    token = self.rfile.readline().decode("ascii").strip()
                try:
                    decoded = base64.b64decode(token).split(b"\0")
                    authenticated = decoded[-2:] == [b"fixture@example.invalid", b"fixture-only-secret"]
                except Exception:
                    authenticated = False
                self.line("235 2.7.0 Authenticated" if authenticated else "535 5.7.8 Invalid fixture credentials")
            elif command == "MAIL" and authenticated:
                recipients = []
                self.line("250 OK")
            elif command == "RCPT" and authenticated:
                recipients.append(value)
                self.line("250 OK")
            elif command == "DATA" and authenticated and recipients:
                self.line("354 Finish with dot")
                data = []
                while True:
                    line = self.rfile.readline()
                    if not line:
                        return
                    if line == b".\r\n":
                        break
                    data.append(line[1:] if line.startswith(b"..") else line)
                with LOCK:
                    DELIVERIES.append((recipients[:], b"".join(data)))
                self.line("250 2.0.0 Accepted locally")
            elif command in ("NOOP", "RSET"):
                self.line("250 OK")
            elif command == "QUIT":
                self.line("221 Bye")
                return
            else:
                self.line("500 Unsupported fixture command")

def run():
    parser = argparse.ArgumentParser()
    parser.add_argument("--vendored", action="store_true", help="Build native-tls with vendored OpenSSL")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="mail-ex-fixture-") as tmp:
        path = Path(tmp)
        def openssl(*command):
            subprocess.run(["openssl", *command], cwd=path, check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
                "-subj", "/CN=Mail EX Fixture CA", "-keyout", "ca.key", "-out", "ca.crt",
                "-addext", "basicConstraints=critical,CA:TRUE")
        openssl("req", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=localhost",
                "-keyout", "server.key", "-out", "server.csr")
        (path / "extensions.cnf").write_text(
            "subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\nbasicConstraints=CA:FALSE\n")
        openssl("x509", "-req", "-in", "server.csr", "-CA", "ca.crt", "-CAkey", "ca.key",
                "-CAcreateserial", "-days", "1", "-out", "server.crt", "-extfile", "extensions.cnf")
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(path / "server.crt", path / "server.key")
        with TLSFixture(IMAP, context) as imap, TLSFixture(SMTP, context) as smtp:
            for server in (imap, smtp):
                threading.Thread(target=server.serve_forever, daemon=True).start()
            env = os.environ.copy()
            env.update(SSL_CERT_FILE=str(path / "ca.crt"),
                       MAIL_EX_FIXTURE_IMAP_PORT=str(imap.server_address[1]),
                       MAIL_EX_FIXTURE_SMTP_PORT=str(smtp.server_address[1]))
            command = ["cargo", "test", "--manifest-path", str(ROOT / "core/Cargo.toml"),
                       "--test", "protocol_fixture"]
            if args.vendored:
                command.extend(["--features", "native-tls/vendored"])
            command.extend(["--", "--ignored", "--nocapture"])
            try:
                subprocess.run(command, cwd=ROOT, env=env, check=True)
                assert len(DELIVERIES) == 1, "Expected exactly one locally accepted SMTP message"
                recipients, payload = DELIVERIES[0]
                assert any("receiver@example.invalid" in r for r in recipients)
                assert any("hidden@example.invalid" in r for r in recipients)
                assert b"Local test only." in payload
                assert not re.search(rb"(?im)^Bcc:", payload), "Bcc leaked into MIME headers"
                print("Local TLS IMAP/SMTP, mailbox MOVE, flags, reply, draft and Bcc fixture passed.")
            finally:
                imap.shutdown()
                smtp.shutdown()

if __name__ == "__main__":
    run()
