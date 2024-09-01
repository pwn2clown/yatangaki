#!/bin/bash
#  https://chromium.googlesource.com/chromium/src/+/master/docs/linux/cert_management.md

PORT="${1}"

certutil -d sql:$HOME/.pki/nssdb -L -n yatangaki_ca > /dev/null ||\
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n yatangaki_ca -i $HOME/.yatangaki/ca.pem

chromium-browser\
    --proxy-server=localhost:$PORT\
    --disable-dinosaur-easter-egg
