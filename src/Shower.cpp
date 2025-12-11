#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>

Shower::Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed)
    : showerTimer(displayDIO, displayCLK), button(button), temperatureGround(temperatureGround), redLed(redLed), greenLed(greenLed)
{
    pinMode(displayDIO, OUTPUT);
    pinMode(displayCLK, OUTPUT);
    pinMode(button, INPUT_PULLUP);
};

u8_t Shower::getTemperatureGround()
{
    return temperatureGround;
}

String WaterTemperature::getInfo()
{
    return ""; // TODO: Написати сюда текст
}

void Shower::setWaterTemperature(CircularBuffer<String, 30> &queue)
{
}

String Shower::getWaterTemperature()
{
    return waterTemperature.getInfo();
}

void Shower::updateDisplay()
{
    showerTimer.setCursor(0);
    if (!digitalRead(button) == false)
    {
        isRunning = false;
        showerTimer.print("FREE");
        showerTimer.update();
        return;
    }
    if (!digitalRead(button) != isRunning)
    {
        isRunning = true;
        start = millis();
    }
    unsigned long totalMs = millis() - start;

    u8_t minutes = (totalMs / (1000 * 60)) % 60;
    u8_t seconds = (totalMs / 1000) % 60;

    char timeBuffer[6];

    sprintf(timeBuffer, "%02d%02d", minutes, seconds);

    showerTimer.print(timeBuffer);
    showerTimer.colon(true);
    showerTimer.update();
};
