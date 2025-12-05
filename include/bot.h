#pragma once
#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>

#define FB_NO_FILE

extern FastBot2 bot;

void updateh(fb::Update &u);
