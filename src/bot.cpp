#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>

FastBot2 bot(BOT_TOKEN);

namespace {

// Дістаємо «гарне» імʼя: @username → first_name → "User <id>"
String pickDisplayName(fb::UserRead user)
{
    String uname = user.username().toString();
    if (uname.length() > 0) return "@" + uname;
    String fname = user.firstName().toString();
    if (fname.length() > 0) return fname;
    return "User " + user.id().toString();
}

bool isCommand(const String &text, const char *cmd)
{
    if (!text.startsWith(cmd)) return false;
    size_t len = strlen(cmd);
    if (text.length() == len) return true;
    char next = text.charAt(len);
    return next == ' ' || next == '@';
}

fb::Message makeReply(const String &body, int32_t messageID, const fb::ID &chatID)
{
    fb::Message msg;
    msg.chatID = chatID;
    msg.reply = createReply(messageID, chatID);
    msg.text = body;
    return msg;
}

} // namespace

void updateh(fb::Update &u)
{
    if (!(u.message().chat().id() == GROUP_ID)) return;

    String text = u.message().text().toString();
    String fromId = u.message().from().id().toString();
    int32_t msgId = u.message().id();
    fb::ID chatID = u.message().chat().id();

    if (isCommand(text, "/start"))
    {
        fb::Message msg("👋 Привіт! Це бот для управління чергою в душ Гуртожитку №1.\n\n"
                        "Доступні команди:\n"
                        "/get_info — загальна інформація\n"
                        "/queue — переглянути всю чергу\n"
                        "/position — твоя позиція в черзі\n"
                        "/join_to_queue — додатися в чергу\n"
                        "/leave_from_queue — вийти з черги\n"
                        "/clear_queue — очистити чергу (тільки адмін)",
                        chatID);
        bot.sendMessage(msg, true);
    }
    else if (isCommand(text, "/get_info"))
    {
        getInfoMessage(msgId, chatID);
    }
    else if (isCommand(text, "/queue"))
    {
        showQueueMessage(msgId, chatID);
    }
    else if (isCommand(text, "/position"))
    {
        showPositionMessage(fromId, msgId, chatID);
    }
    else if (isCommand(text, "/join_to_queue"))
    {
        if (!smartShower.isWorkingTime())
        {
            bot.sendMessage(makeReply("❌ Зараз не робочий час.", msgId, chatID), true);
            return;
        }
        addToQueueMessage(fromId, pickDisplayName(u.message().from()), msgId, chatID);
    }
    else if (isCommand(text, "/leave_from_queue"))
    {
        queueReductionMessage(fromId, msgId, chatID);
    }
    else if (isCommand(text, "/clear_queue"))
    {
        clearQueueMessage(fromId, msgId, chatID);
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
    String body;
    body.reserve(200);
    body  = "📊 Загальна інформація:\n\n";
    body += "🚿 Душ 1:\n" + shower1.getWaterTemperature() + "\n\n";
    body += "🚿 Душ 2:\n" + shower2.getWaterTemperature() + "\n\n";
    body += "Черга: " + String(smartShower.queueLen());
    bot.sendMessage(makeReply(body, messageID, chatID), true);
}

void notifyNextInQueue(const fb::ID chatID)
{
    QueueHead head = smartShower.getHead();
    if (head.isEmpty || head.id == "0") return;
    fb::Message msg;
    msg.mode = fb::Message::Mode::MarkdownV2;
    msg.text = "🔔 Твоя черга наступна\\! [" + head.displayName + "](tg://user?id=" + head.id + ")";
    msg.chatID = chatID;
    bot.sendMessage(msg, true);
}

void queueReductionMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    int8_t indexInQueue = smartShower.isInQueue(id);
    if (indexInQueue == -1)
    {
        smartShower.requestBeep(true);
        bot.sendMessage(makeReply("❌ Вас немає в черзі.", messageID, chatID), true);
        return;
    }
    bool wasFirst = (indexInQueue == 0);
    smartShower.queueReduction(id);
    smartShower.requestBeep(false);
    bot.sendMessage(makeReply("✅ Ви успішно вийшли з черги.", messageID, chatID), true);
    if (wasFirst) notifyNextInQueue(chatID);
}

void addToQueueMessage(const String &id, const String &name,
                       const int32_t messageID, const fb::ID chatID)
{
    int8_t position = smartShower.isInQueue(id);
    if (position != -1)
    {
        smartShower.requestBeep(true);
        String text = "⚠️ Ви вже в черзі! Ваша позиція: " + String(position + 1);
        bot.sendMessage(makeReply(text, messageID, chatID), true);
        return;
    }
    if (!smartShower.addingToQueue(id, name))
    {
        smartShower.requestBeep(true);
        bot.sendMessage(makeReply("❌ Черга заповнена, спробуйте пізніше.", messageID, chatID), true);
        return;
    }
    smartShower.requestBeep(false);
    int8_t newPos = smartShower.isInQueue(id) + 1;
    String text = "✅ " + name + ", ви додані в чергу! Ваша позиція: " + String(newPos);
    bot.sendMessage(makeReply(text, messageID, chatID), true);
}

void showQueueMessage(const int32_t messageID, const fb::ID chatID)
{
    uint8_t len = smartShower.queueLen();
    if (len == 0)
    {
        bot.sendMessage(makeReply("📭 Черга порожня.", messageID, chatID), true);
        return;
    }

    String body;
    body.reserve(32 + len * 24);
    body = "📋 Черга (" + String(len) + "):\n";
    for (uint8_t i = 0; i < len; i++)
    {
        QueueEntry entry = smartShower.getQueueAt(i);
        body += String(i + 1) + ". " + entry.displayName() + "\n";
    }
    bot.sendMessage(makeReply(body, messageID, chatID), true);
}

void showPositionMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    int8_t pos = smartShower.isInQueue(id);
    String text = (pos == -1)
        ? String("ℹ️ Вас немає в черзі. /join_to_queue щоб додатися.")
        : ("📍 Ваша позиція: " + String(pos + 1) + " із " + String(smartShower.queueLen()));
    bot.sendMessage(makeReply(text, messageID, chatID), true);
}

void clearQueueMessage(const String &requester, const int32_t messageID, const fb::ID chatID)
{
    if (requester != String(ADMIN_ID))
    {
        smartShower.requestBeep(true);
        bot.sendMessage(makeReply("🚫 Ця команда доступна тільки адміну.", messageID, chatID), true);
        return;
    }
    smartShower.clearQueue();
    smartShower.requestBeep(false);
    bot.sendMessage(makeReply("🧹 Чергу очищено.", messageID, chatID), true);
}
