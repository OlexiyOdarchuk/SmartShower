#pragma once
#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <WiFi.h>
#include <bot.hpp>
#include <NTPClient.h>
#include <Shower.hpp>

extern Shower shower1;
extern Shower shower2;
extern WiFiUDP ntpUPD;
extern NTPClient timeClient;