#!/usr/bin/env python3

import smtplib

def send(username, password, host, fromaddr, toaddr, msg):
    s=smtplib.SMTP(host=host, port=25)
    s.starttls()
    s.login(username, password)
    msgout = f"From: {fromaddr}\r\nTo: {toaddr}\r\n\r\n{msg}\n"
    s.sendmail(fromaddr, toaddr, msgout)

if __name__ == "__main__":
    username = ""
    password = ""
    host = ""
    fromaddr = ""
    toaddr = ""
    msg = "Testing!"

    send(username, password, host, fromaddr, toaddr, msg)
