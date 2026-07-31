#pragma once

#include <3ds/types.h>

void task_init(void);
void task_exit(void);
bool task_is_quit_all(void);
Handle task_get_pause_event(void);
