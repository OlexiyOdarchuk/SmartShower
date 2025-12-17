#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>
#include <NTPClient.h>
#include <GyverOLED.h>
#include <SmartShower.hpp>

WiFiUDP ntpUPD;
NTPClient timeClient(ntpUPD, "0.ua.pool.ntp.org", 2 * 3600, 60000);
Shower shower1(SHOWER_1_TIMER_DIO, SHOWER_1_TIMER_CLK, SHOWER_1_BUTTON, SHOWER_1_GROUND, SHOWER_1_RED_LED, SHOWER_1_GREEN_LED);
Shower shower2(SHOWER_2_TIMER_DIO, SHOWER_2_TIMER_CLK, SHOWER_2_BUTTON, SHOWER_2_GROUND, SHOWER_2_RED_LED, SHOWER_2_GREEN_LED);
GyverOLED<SSD1306_128x32, OLED_NO_BUFFER> oled;
SmartShower smartShower(bot, shower1, shower2, oled, SHOWER_TEMPERATURE_BUTTON_1, SHOWER_TEMPERATURE_BUTTON_2, SHOWER_TEMPERATURE_BUTTON_3, SHOWER_TEMPERATURE_BUTTON_4);