#!/usr/bin/env python3

import requests
import logging

class ChannelBuilderTester:
    def __init__(self, host, api_port, frontend_port,
            fresh_username, fresh_password, use_https = False
        ):
        self.sess_keys = dict()
        self.use_https = use_https
        self.host = host
        self.api_port = api_port
        self.username = fresh_username
        self.password = fresh_password
        self.frontend_port = frontend_port

    def get_origin(self):
        http_trail = "s" if self.use_https else ""
        return f"http{ http_trail }://{ self.host }:{ self.frontend_port }"

    def get_referer(self):
        return f"{ self.get_origin() }/"

    def get_url(self, endpoint):
        http_trail = "s" if self.use_https else ""
        return f"http{ http_trail }://{ self.host }:{ self.api_port }/api/v1/{ endpoint }"

    def create_fresh_account(self):
        data = { "username": self.username, "password": self.password, }
        headers = { "referer": self.get_referer(), }
        req = requests.post(self.get_url("create_account"),
            headers = headers,
            data = data,
        )
        notice = "" if req.status_code == 200 else "Creation didn't return 200.  "
        input(f"{notice}Press enter when you've confirmed the account.")
        return req.status_code == 200

    def authenticate_fe(self, username = None, password = None):
        return self.authenticate("fe", username, password)

    def authenticate_ro(self, username = None, password = None):
        return self.authenticate("ro", username, password)

    def authenticate(self, portion, username = None, password = None):
        if username is None:
            username = self.username
            password = self.password
        data = { "username": username, "password": password, }
        headers = { "referer": self.get_referer(), }
        req = requests.post(self.get_url(f"authenticate_{ portion }"),
            headers = headers,
            data = data,
        )
        #print(req.text)
        print(req.headers)
        #print(req.status_code)
        if req.status_code == 200:
            set_cookie = req.headers["set-cookie"]
            cook_end = set_cookie.find(";")
            self.sess_keys[(username, portion)] = set_cookie[:cook_end]
        return req.status_code == 200

    def get_sess_key_fe(self, username = None, password = None):
        return self.get_sess_key("fe", username, password)

    def get_sess_key_ro(self, username = None, password = None):
        return self.get_sess_key("ro", username, password)

    def get_sess_key(self, portion, username = None, password = None):
        if username is None:
            username = self.username
            password = self.password
        if (username, portion) not in self.sess_keys:
            if not self.authenticate(portion, username = username, password = password):
                logging.error("Failed to authenticate and get session key")
                return ""
        return self.sess_keys[(username, portion)]

    def get_channel_xml_roku(self):
        sess_key = self.get_sess_key_ro()
        headers = { "referer": self.get_referer(), "Cookie": sess_key, }
        print(headers)
        req = requests.get(self.get_url("get_channel_xml_ro"),
            headers = headers,
        )
        print(req.text)
        return req.status_code == 200
        

if __name__ == "__main__":
    host = "192.168.86.11"
    api_port = "3031"
    frontend_port = "8080"
    username = "runningstreamllc+test10@gmail.com"
    password = "12345"

    tester = ChannelBuilderTester(host, api_port, frontend_port, username, password)
    tests = [
        ("Create Account", tester.create_fresh_account),
        ("Authenticate Frontend", tester.authenticate_fe),
        ("Authenticate Roku", tester.authenticate_ro),
        ("Get Channel XML Roku", tester.get_channel_xml_roku),
    ]

    results = [(name, func()) for (name, func) in tests]

    for (name, result) in results:
        print(f"{ name }: { result }")
