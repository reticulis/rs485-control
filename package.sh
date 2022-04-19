#!/bin/bash

mv package rs485-control

export DLLS=`pds rs485-control/*.exe -f`
for DLL in $DLLS; do
    cp "$DLL" rs485-control
	# cp "$DLL" package
done

cp /usr/x86_64-w64-mingw32/sys-root/mingw/bin/gdbus.exe rs485-control


mkdir -p rs485-control/share/{themes,gtk-4.0,glib-2.0}

find rs485-control -maxdepth 1 -type f -exec mingw-strip {} +

zip -qr rs485-control.zip rs485-control/*
