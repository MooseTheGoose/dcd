#!/bin/sh

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <package-name>"
    exit 1
fi
CMD="adb shell ps -A | grep '$1' | awk '"
CMD+='{print $2}'
CMD+="'"
APP_PID=`$CMD`
if [ "$APP_PID" = "" ]; then
    echo "No PID found"
    exit 1
fi
if [ `echo "$APP_PID" | wc -l` -gt 1 ]; then
   echo "Found multiple apps with the package name"
   exit 1
fi
adb forward --remove-all
adb forward tcp:4444 "jdwp:$APP_PID"

