#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <3ds.h>

int main(int argc, char* argv[])
{
	gfxInitDefault();
	consoleInit(GFX_TOP, NULL);
	nsInit();

	dlpClntInit();

	u16 channel = 0;
	DLPCLNT_GetChannel(&channel);

	printf("Download Play Client Example\n");
	printf("Press A to scan.\n");
	printf("Press B to stop scan or leave dlp session.\n");
	printf("Press start to exit.\n");

	dlpClntMyStatus status;
	dlpClntState prevState = 0;

	bool scanning = false;

	dlpClntTitleInfo title;

	// Main loop
	while (aptMainLoop())
	{
		gspWaitForVBlank();
		gfxSwapBuffers();
		hidScanInput();

		u32 kDown = hidKeysDown();

		DLPCLNT_GetMyStatus(&status);

		if (status.state != prevState) {
			printf("DLP State: %d\n", status.state);
			prevState = status.state;
		}

		if (scanning && dlpClntWaitForEvent(false, false)) {
			printf("Found download play server! Connecting...\n");
			scanning = false;
			DLPCLNT_StopScan();
			DLPCLNT_GetTitleInfoInOrder(&title, NULL);

			DLPCLNT_GetTitleInfo(&title, NULL, title.macAddr, title.uniqueId, title.variation);
			DLPCLNT_StartTitleDownload(title.macAddr, title.uniqueId, title.variation);

			printf("Connected. Waiting for server to begin game.\n");
		}
		else if (status.state == DLPCLNT_STATE_DOWNLOADING) {
			printf("\x1b[29;1HDownload progress:     %6.2f%%\x1b[K\n", (float)(status.unitsRecvd * 100) / (float)status.unitsTotal);
		}
		else if (status.state == DLPCLNT_STATE_COMPLETE) { // Title finished downloading
			u8 passphrase[9];
			DLPCLNT_GetWirelessRebootPassphrase(passphrase);

			NS_SetWirelessRebootInfo(title.macAddr, passphrase);

			aptSetChainloader(dlpCreateChildTid(title.uniqueId, title.variation), 0);
			break;
		}

		if (kDown & KEY_A) {
			dlpClntWaitForEvent(false, false); // Clear any earlier events.
			if (R_SUCCEEDED(DLPCLNT_StartScan(channel, NULL, 0))) {
				scanning = true;
				printf("Began scanning.\n");
			}
		}
		else if (kDown & KEY_B) {
			if (status.state >= DLPCLNT_STATE_JOINED) {
				if (R_SUCCEEDED(DLPCLNT_StopSession())) {
					printf("Left session.\n");
				}
			}
			else if (status.state == DLPCLNT_STATE_SCANNING) {
				if (R_SUCCEEDED(DLPCLNT_StopScan())) {
					scanning = false;
					printf("Stopped scanning.\n");
				}
			}
		}

		if (kDown & KEY_START) {
			break; // break in order to return to hbmenu
		}
	}

	dlpClntExit();
	gfxExit();
	return 0;
}
