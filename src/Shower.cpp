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
    if (temperature == 0)
    {
        return "Температура не встановлена";
    }
    String info = "Температура: " + String(temperature) + "\n";
    info += "Час: " + time + "\n";
    if (user == "0" || user == "")
    {
        info += "Користувач: Зареєстровано кнопкою";
    }
    else
    {
        info += "Користувач: " + user;
    }
    return info;
}

void Shower::setWaterTemperature(const u8_t temperature)
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
    // Для INPUT_PULLUP: LOW = натиснуто (зайнятий), HIGH = відпущено (вільний)
    bool buttonPressed = (digitalRead(button) == LOW);
    
    if (isBusy == buttonPressed)
    {
        return false; // Стан не змінився
    }
    else
    {
        isBusy = buttonPressed;
        ledControl();
        return true; // Стан змінився
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

bool Shower::isBusyNow()
{
    return isBusy;
}

String Shower::getWhoNow()
{
    return whoNow;
}
