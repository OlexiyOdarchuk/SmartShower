#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>
#include <logic.h>
#include <NTPClient.h>

WiFiUDP ntpUPD;
NTPClient timeClient(ntpUPD, "0.ua.pool.ntp.org", 2 * 3600, 60000);