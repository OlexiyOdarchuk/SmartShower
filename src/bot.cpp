#include <Arduino.h>
#include <secrets.hpp>
#include <FastBot2.h>
#include <WiFi.h>
#include <bot.hpp>
#include <logic.hpp>

FastBot2 bot(BOT_TOKEN);

namespace {

String pickDisplayName(fb::UserRead user)
{
    String uname = user.username().toString();
    if (uname.length() > 0) return "@" + uname;
    String fname = user.firstName().toString();
    if (fname.length() > 0) return fname;
    return "User " + user.id().toString();
}

String htmlEscape(const String &in)
{
    String out;
    out.reserve(in.length() + 8);
    for (size_t i = 0; i < in.length(); i++)
    {
        char c = in.charAt(i);
        switch (c)
        {
        case '&': out += "&amp;";  break;
        case '<': out += "&lt;";   break;
        case '>': out += "&gt;";   break;
        default:  out += c;        break;
        }
    }
    return out;
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

void send(const fb::Message &msg)
{
    if (!bot.sendMessage(const_cast<fb::Message &>(msg), true))
    {
        Serial.println("[Bot] sendMessage failed");
    }
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
        send(msg);
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
    send(makeReply(smartShower.infoReport(), messageID, chatID));
}

void notifyNextInQueue(const fb::ID chatID)
{
    QueueHead head = smartShower.getHead();
    if (head.isEmpty || head.id == "0") return;
    // HTML-режим + екранування: ім'я користувача може містити _ * [ ] тощо
    // (валідні в Telegram username/first_name), що ламало б MarkdownV2-розмітку.
    fb::Message msg;
    msg.mode = fb::Message::Mode::HTML;
    msg.text = "🔔 Твоя черга наступна! <a href=\"tg://user?id=" + head.id + "\">"
             + htmlEscape(head.displayName) + "</a>";
    msg.chatID = chatID;
    send(msg);
}

void queueReductionMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    bool wasFirst = false;
    if (!smartShower.leaveQueue(id, wasFirst))
    {
        smartShower.requestBeep(true);
        send(makeReply("❌ Вас немає в черзі.", messageID, chatID));
        return;
    }
    smartShower.requestBeep(false);
    send(makeReply("✅ Ви успішно вийшли з черги.", messageID, chatID));
    if (wasFirst) notifyNextInQueue(chatID);
}

void addToQueueMessage(const String &id, const String &name,
                       const int32_t messageID, const fb::ID chatID)
{
    JoinResult r = smartShower.tryJoin(id, name);
    switch (r.status)
    {
    case JoinResult::OFF_HOURS:
        smartShower.requestBeep(true);
        send(makeReply("❌ Зараз не робочий час.", messageID, chatID));
        return;
    case JoinResult::ALREADY_IN:
        smartShower.requestBeep(true);
        send(makeReply("⚠️ Ви вже в черзі! Ваша позиція: " + String(r.position), messageID, chatID));
        return;
    case JoinResult::FULL:
        smartShower.requestBeep(true);
        send(makeReply("❌ Черга заповнена, спробуйте пізніше.", messageID, chatID));
        return;
    case JoinResult::ADDED:
        smartShower.requestBeep(false);
        send(makeReply("✅ " + name + ", ви додані в чергу! Ваша позиція: " + String(r.position),
                       messageID, chatID));
        return;
    }
}

void showQueueMessage(const int32_t messageID, const fb::ID chatID)
{
    QueueEntry entries[30];
    uint8_t count = smartShower.snapshotQueue(entries, 30);
    if (count == 0)
    {
        send(makeReply("📭 Черга порожня.", messageID, chatID));
        return;
    }

    String body;
    body.reserve(32 + count * 24);
    body = "📋 Черга (" + String(count) + "):\n";
    for (uint8_t i = 0; i < count; i++)
    {
        body += String(i + 1) + ". " + entries[i].displayName() + "\n";
    }
    send(makeReply(body, messageID, chatID));
}

void showPositionMessage(const String &id, const int32_t messageID, const fb::ID chatID)
{
    int8_t pos = smartShower.isInQueue(id);
    String text = (pos == -1)
        ? String("ℹ️ Вас немає в черзі. /join_to_queue щоб додатися.")
        : ("📍 Ваша позиція: " + String(pos + 1) + " із " + String(smartShower.queueLen()));
    send(makeReply(text, messageID, chatID));
}

void clearQueueMessage(const String &requester, const int32_t messageID, const fb::ID chatID)
{
    if (requester != String(ADMIN_ID))
    {
        smartShower.requestBeep(true);
        send(makeReply("🚫 Ця команда доступна тільки адміну.", messageID, chatID));
        return;
    }
    smartShower.clearQueue();
    smartShower.requestBeep(false);
    send(makeReply("🧹 Чергу очищено.", messageID, chatID));
}
