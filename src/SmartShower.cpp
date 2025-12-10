#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <Shower.hpp>
#include <SmartShower.hpp>
#include <CircularBuffer.hpp>

SmartShower::SmartShower(FastBot2 &bot, Shower &shower1, Shower &shower2) : bot(bot), shower1(shower1), shower2(shower2) {}

void SmartShower::addingToQueue(String &id)
{
    if (!queue.isFull())
    {
        queue.push(id);
    }
}

void SmartShower::queueReduction(Shower &shower)
{
    if (!queue.isEmpty()) // ! Винести це перед викликом!!!
    {
        String element = queue.shift();
        if (queue.first() != "0")
        {
            fb::Message msg;
            msg.text = "" + queue.first();
            bot.sendMessage(msg);
            // TODO: Перенести на форматування і написати нормальний текст
        }
    }
}

void SmartShower::pressShowerButton(Shower &shower)
{
    queueReduction(shower);
}