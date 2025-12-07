#pragma once
#include <Arduino.h>
#include <secrets.h>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.h>
#include <Shower.h>

class SmartShower
{
public:
private:
    FastBot2 bot;
    Shower shower1;
    Shower shower2;
};