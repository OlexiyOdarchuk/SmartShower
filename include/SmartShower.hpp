#pragma once
#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <CircularBuffer.hpp>

class SmartShower
{
public:
    SmartShower(FastBot2 &bot, Shower &shower1, Shower &shower2, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4);

private:
    CircularBuffer<String, 30> queue;
    FastBot2 bot;
    Shower shower1;
    Shower shower2;
    u8_t temperatureButtons[4];
    u8_t temperatureGrounds[2];
    void addingToQueue(String &id);
    void queueReduction(Shower &shower);
    void pressShowerButton(Shower &shower);
    void updateTemperatureButtons();
};