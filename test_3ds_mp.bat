@echo off
echo === Rabuka 3DS Multiplayer Test ===
echo.
echo Starting HOST instance...
start "Azahar Host" "C:\azahar_host\azahar.exe"
echo Starting CLIENT instance...
start "Azahar Client" "C:\azahar_client\azahar.exe"
echo.
echo Both instances launched!
echo 1. Set username in Settings - System on each
echo 2. Host: Multiplayer - Host Room (Private)
echo 3. Client: Multiplayer - Join Room (127.0.0.1:24872)
echo 4. Both: File - Load - output_3ds\rabuka_3ds.3dsx
echo 5. Both: Local Multiplayer - pick deck
echo    Host selects Host role, Client selects Client role
pause
