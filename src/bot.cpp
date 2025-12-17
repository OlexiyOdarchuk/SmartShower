#include <Arduino.h>
#include <secrets.hpp>
#include <GyverSegment.h>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>

FastBot2 bot(BOT_TOKEN);

void updateh(fb::Update &u)
{
    if (u.message().chat().id() == GROUP_ID)
    {
        u8_t hour = static_cast<u8_t>(timeClient.getHours());
        if ((hour > NIGHT_TIME_START && hour < NIGHT_TIME_FINISH) || (hour > MIDDAY_TIME_START && hour < MIDDAY_TIME_FINISH))
        {
            if (u.message().text() == "/start")
            {
                fb::Message msg("Hello!", GROUP_ID);
                bot.sendMessage(msg, true);
            }
            if (u.message().text() == "/get_info")
            {
                getInfoMessage(u.message().id(), u.message().chat().id());
            }
            if (u.message().text() == "/joid_to_queue")
            {
                addToQueueMessage(u.message().from().id(), u.message().id(), u.message().chat().id());
            }
            if (u.message().text() == "/leave_from_queue")
            {
                queueReductionMessage(u.message().from().id(), u.message().id(), u.message().chat().id());
            }
        }
    }
}

fb::ReplyParam createReply(const int32_t messageID, const fb::ID chatID)
{
    fb::ReplyParam reply;
    reply.messageID = messageID;
    reply.chatID = chatID;
    return reply;
}

void getInfoMessage(const int32_t messageID, const fb::ID chatID)
{
    String wt1 = shower1.getWaterTemperature();
    String wt2 = shower2.getWaterTemperature();
    fb::Message msg;
    msg.reply = createReply(messageID, chatID);
    msg.text = ""; // TODO: Написати сюди повідомлення з інформацією
    bot.sendMessage(msg, true);
}

void queueReductionMessage(const fb::ID chatID)
{
    String first = smartShower.getFirstId();
    if (first != "0" && first != "-1")
    {
        fb::Message msg;
        msg.mode = fb::Message::Mode::MarkdownV2;
        msg.text = "Привіт, [користувачу](tg://user?id=" + first + ")"; // TODO: написати текст, що готовий душ
        msg.chatID = chatID;
        bot.sendMessage(msg, true);
    }
}

void queueReductionMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    fb::Message msg;
    msg.chatID = chatID;
    msg.reply = createReply(messageID, chatID);
    int8_t indexInQueue = smartShower.isInQueue(id);
    if (indexInQueue == -1)
    {
        msg.text = ""; // TODO: Написати повідомлення, що вас немає в списку;
        bot.sendMessage(msg, true);
        return;
    }
    smartShower.queueReduction(id);
    msg.text = ""; // TODO: Написати повідомлення про успішне вилучення з черги
    bot.sendMessage(msg, true);

    queueReductionMessage(chatID);
}

void addToQueueMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    fb::Message msg;
    msg.chatID = chatID;
    msg.reply = createReply(messageID, chatID);
    if (smartShower.addingToQueue(id))
    {
        msg.text = ""; // TODO: Дописати текст на успішне додавання в чергу
        bot.sendMessage(msg, true);
        return;
    }
    msg.text = ""; // TODO: Дописати текст на неуспішне додавання в чергу
    bot.sendMessage(msg, true);
}
