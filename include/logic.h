#pragma once
#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <WiFi.h>
#include <bot.h>
#include <NTPClient.h>

extern WiFiUDP ntpUPD;
extern NTPClient timeClient;