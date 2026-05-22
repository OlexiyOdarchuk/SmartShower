#pragma once
#include <Arduino.h>
#include <Shower.hpp>
#include <GyverOLED.h>
#include <SmartShower.hpp>

extern Shower shower1;
extern Shower shower2;
extern GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> oled;
extern SmartShower smartShower;
