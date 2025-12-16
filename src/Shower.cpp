#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>
#include <Shower.hpp>

Shower::Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed)
    : showerTimer(displayDIO, displayCLK), button(button), temperatureGround(temperatureGround), redLed(redLed), greenLed(greenLed)
{
    pinMode(displayDIO, OUTPUT);
    pinMode(displayCLK, OUTPUT);
    pinMode(button, INPUT_PULLUP);
    pinMode(redLed, OUTPUT);
    pinMode(greenLed, OUTPUT);
    waterTemperature.temperature = 0;
    waterTemperature.user = "";
    waterTemperature.time = "";
};

void Shower::ledControl()
{
    if (isBusy)
    {
        digitalWrite(redLed, HIGH);
        digitalWrite(greenLed, LOW);
    }
    else
    {
        digitalWrite(redLed, LOW);
        digitalWrite(greenLed, HIGH);
    }
}

u8_t Shower::getTemperatureGround()
{
    return temperatureGround;
}

String WaterTemperature::getInfo()
{
    return ""; // TODO: Написати сюда текст
}

void Shower::setWaterTemperature(const CircularBuffer<String, 30> &queue, u8_t const temperature)
{
    waterTemperature.temperature = temperature;
    waterTemperature.time = timeClient.getFormattedTime();
    waterTemperature.user = whoNow;
}

String Shower::getWaterTemperature()
{
    return waterTemperature.getInfo();
}

bool Shower::getChange()
{
    if (isBusy == digitalRead(button))
    {
        return false;
    }
    else
    {
        isBusy = digitalRead(button);
        ledControl();
        return true;
    }
}

void Shower::updateDisplay()
{
    isChange = getChange();
    showerTimer.setCursor(0);
    if (isChange && !isBusy)
    {
        showerTimer.print("FREE");
        showerTimer.update();
        return;
    }
    if (isBusy && isChange)
    {
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

void Shower::setWhoNow(const String &id)
{
    whoNow = id;
}

void Shower::getTemperatureButtons(const u8_t buttons[4])
{
}
