#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>

Shower::Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed)
    : showerTimer(displayDIO, displayCLK), button(button), temperatureGround(temperatureGround), redLed(redLed), greenLed(greenLed) {};

Shower::Shower(u8_t displayDIO, u8_t displayCLK, u8_t button, u8_t temperatureGround, u8_t redLed, u8_t greenLed, u8_t temperatureButton1, u8_t temperatureButton2, u8_t temperatureButton3, u8_t temperatureButton4)
    : showerTimer(displayDIO, displayCLK), button(button), temperatureGround(temperatureGround), redLed(redLed), greenLed(greenLed), temperatureButtons{temperatureButton1, temperatureButton2, temperatureButton3, temperatureButton4} {};

String WaterTemperature::getInfo()
{
    return ""; // TODO: Написати сюда текст
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

// void Shower::updateDisplay()
// {
//     showerTimer.setCursor(0);
//     if (!digitalRead(button) == false)
//     {
//         isRunning = false;
//         showerTimer.print("FREE");
//         showerTimer.update();
//         return;
//     }
//     if (!digitalRead(button) != isRunning)
//     {
//         isRunning = true;
//         start = millis();
//     }
//     unsigned long totalMs = millis() - start;

//     u8_t minutes = (totalMs / (1000 * 60)) % 60;
//     u8_t seconds = (totalMs / 1000) % 60;

//     char timeBuffer[6];

//     sprintf(timeBuffer, "%02d%02d", minutes, seconds);

//     showerTimer.print(timeBuffer);
//     showerTimer.colon(true);
//     showerTimer.update();
// };

// TODO
// void Shower::setWaterTemperature()
// {
//     for (u8_t a : temperatureButtons)
//     {
//         if (a =)
//     }
// }
