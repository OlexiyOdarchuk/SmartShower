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
        if (u.message().text() == "/start")
        {
            fb::Message msg("👋 Привіт! Це бот для управління чергою в душ Гуртожитку №1.\n\nДоступні команди:\n/get_info - інформація про душеві кабіни\n/joid_to_queue - додатися в чергу\n/leave_from_queue - вийти з черги", GROUP_ID);
            bot.sendMessage(msg, true);
        }
        if (u.message().text() == "/get_info")
        {
            getInfoMessage(u.message().id(), u.message().chat().id());
        }
        if (u.message().text() == "/joid_to_queue" || u.message().text() == "/join_to_queue")
        {
            // Перевірка робочого часу
            if (smartShower.isWorkingTime())
            {
                addToQueueMessage(u.message().from().id(), u.message().id(), u.message().chat().id());
            }
            else
            {
                fb::Message msg;
                msg.chatID = u.message().chat().id();
                msg.reply = createReply(u.message().id(), u.message().chat().id());
                msg.text = "❌ Зараз не робочий час. Робочий час визначається налаштуваннями системи.";
                bot.sendMessage(msg, true);
            }
        }
        if (u.message().text() == "/leave_from_queue")
        {
            queueReductionMessage(u.message().from().id(), u.message().id(), u.message().chat().id());
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
    msg.text = "📊 Інформація про душеві кабіни:\n\n";
    msg.text += "🚿 Душ 1:\n" + wt1 + "\n\n";
    msg.text += "🚿 Душ 2:\n" + wt2;
    bot.sendMessage(msg, true);
}

void queueReductionMessage(const fb::ID chatID)
{
    String first = smartShower.getFirstId();
    if (first != "0" && first != "-1")
    {
        fb::Message msg;
        msg.mode = fb::Message::Mode::MarkdownV2;
        msg.text = "🔔 Твоя черга наступна\\! [Користувач](tg://user?id=" + first + ")";
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
        msg.text = "❌ Вас немає в черзі";
        bot.sendMessage(msg, true);
        return;
    }
    smartShower.queueReduction(id);
    msg.text = "✅ Ви успішно вийшли з черги";
    bot.sendMessage(msg, true);

    queueReductionMessage(chatID);
}

void addToQueueMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    fb::Message msg;
    msg.chatID = chatID;
    msg.reply = createReply(messageID, chatID);
    
    // Перевірка, чи користувач вже в черзі
    int8_t position = smartShower.isInQueue(id);
    if (position != -1)
    {
        msg.text = "⚠️ Ви вже в черзі! Ваша позиція: " + String(position + 1);
        bot.sendMessage(msg, true);
        return;
    }
    
    if (smartShower.addingToQueue(id))
    {
        position = smartShower.isInQueue(id) + 1;
        msg.text = "✅ Ви додані в чергу! Ваша позиція: " + String(position);
        bot.sendMessage(msg, true);
        return;
    }
    msg.text = "❌ Черга заповнена, спробуйте пізніше";
    bot.sendMessage(msg, true);
}
