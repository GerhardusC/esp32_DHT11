#include "driver/gpio.h"

void clear_shift_register();
void setup_shift_register();
void toggle_shift_register_clock();
void write_to_shift_register(uint8_t val);