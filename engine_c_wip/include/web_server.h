#ifndef RABUKA_WEB_SERVER_H
#define RABUKA_WEB_SERVER_H

#include "rabuka.h"

int rb_web_server_run(const char *host, int port, const char *web_root, const char *data_dir);

#endif
