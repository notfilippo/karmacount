use std::{collections::HashSet, fmt::Display, str::FromStr, sync::Arc};

use anyhow::Result;
use bincode::{deserialize, serialize};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use teloxide::{
    adaptors::DefaultParseMode,
    payloads::{AnswerCallbackQuerySetters, SendMessageSetters},
    requests::{Requester, ResponseResult},
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message, UserId},
    Bot,
};

use super::{mention_chat, mention_user};
use crate::{
    business::{self, DEFAULT_DOWN, DEFAULT_UP, GRAPH_MAX_SIZE},
    db::{Measure, Store},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Karma {
    Up,
    Down,
}

impl Display for Karma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Karma::Up => write!(f, "+"),
            Karma::Down => write!(f, "-"),
        }
    }
}

impl FromStr for Karma {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('+') {
            Ok(Karma::Up)
        } else if s.starts_with('-') {
            Ok(Karma::Down)
        } else {
            Err(())
        }
    }
}

async fn message_handler_internal(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    msg: Message,
) -> Result<()> {
    if let Some(Ok(modifier)) = msg.text().map(Karma::from_str) {
        if let Some(reply) = msg.reply_to_message() {
            if let (Some(giver), Some(receiver)) = (msg.from(), reply.from()) {
                if !giver.is_bot && !receiver.is_bot && giver.id != receiver.id {
                    let last_karma_timestamp = db.last.get_or(giver.id.to_string(), 0)?;

                    if business::is_assignable_karma_expired(last_karma_timestamp) {
                        db.up.remove(giver.id.to_string())?;
                        db.down.remove(giver.id.to_string())?;
                    }

                    let (db_available, default_available) = match modifier {
                        Karma::Up => (&db.up, DEFAULT_UP),
                        Karma::Down => (&db.down, DEFAULT_DOWN),
                    };

                    let available_current =
                        db_available.get_or(giver.id.to_string(), default_available)?;

                    if available_current < 1 {
                        let keyboard_text =
                            format!("use my karma as {} for {}", modifier, receiver.full_name());
                        let keyboard = InlineKeyboardMarkup::default().append_row(vec![
                            InlineKeyboardButton::callback(
                                keyboard_text,
                                base64::encode(serialize(&(modifier.clone(), receiver.id))?),
                            ),
                        ]);

                        let text = format!("<i>no more {} points available today</i>", modifier);

                        let last_message_key = format!("{}-status", msg.chat.id);
                        if let Some(last_message) = db.last_message.get(&last_message_key)? {
                            bot.delete_message(msg.chat.id, last_message).await.ok();
                        }

                        let update_message = bot
                            .send_message(msg.chat.id, text)
                            .reply_markup(keyboard)
                            .await?;

                        db.last_message
                            .insert(&last_message_key, update_message.id)?;

                        return Ok(());
                    }

                    let available = available_current - 1;
                    db_available.insert(giver.id.to_string(), available)?;

                    let timestamp = Utc::now().timestamp();
                    db.last.insert(giver.id.to_string(), timestamp)?;

                    let karma_current = db.karma.get_or(receiver.id.to_string(), 0)?;

                    let karma = match modifier {
                        Karma::Up => karma_current + 1,
                        Karma::Down => karma_current - 1,
                    };

                    let mut graph = db.graph.get_or(receiver.id.to_string(), vec![])?;
                    graph.push(Measure::new(karma));

                    if graph.len() > GRAPH_MAX_SIZE {
                        let diff = graph.len() - GRAPH_MAX_SIZE;
                        db.graph
                            .insert(receiver.id.to_string(), graph[diff..].to_vec())?;
                    } else {
                        db.graph.insert(receiver.id.to_string(), graph)?;
                    }

                    db.karma.insert(receiver.id.to_string(), karma)?;

                    let mut members = db.members.get_or(msg.chat.id.to_string(), HashSet::new())?;

                    members.insert(giver.id);
                    members.insert(receiver.id);

                    db.members.insert(msg.chat.id.to_string(), members)?;

                    let last_message_key = format!("{}-{}", msg.chat.id, receiver.id);
                    if let Some(last_message) = db.last_message.get(&last_message_key)? {
                        bot.delete_message(msg.chat.id, last_message).await.ok();
                    }

                    let text = format!("reputation of {} ({})", mention_user(receiver), karma);

                    let update_message = bot.send_message(msg.chat.id, text).await?;
                    db.last_message
                        .insert(&last_message_key, update_message.id)?;
                }
            }
        }
    }

    Ok(())
}

pub async fn message_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    msg: Message,
) -> ResponseResult<()> {
    match message_handler_internal(bot, db, msg).await {
        Ok(_) => Ok(()),
        Err(err) => match err.downcast::<teloxide::RequestError>() {
            Ok(err) => Err(err),
            Err(err) => {
                log::error!("Generic error: {}", err);
                Ok(())
            }
        },
    }
}

async fn callback_handler_internal(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    cq: CallbackQuery,
) -> Result<()> {
    if let Some(data) = cq.data {
        let giver = cq.from;
        let (modifier, receiver_id): (Karma, UserId) = deserialize(&base64::decode(data).unwrap())?;

        let karma_receiver_current = db.karma.get_or(receiver_id.to_string(), 0)?;

        let karma_receiver = match modifier {
            Karma::Up => karma_receiver_current + 1,
            Karma::Down => karma_receiver_current - 1,
        };

        let last_karma_timestamp = db.last.get_or(giver.id.to_string(), 0)?;

        if business::is_assignable_karma_expired(last_karma_timestamp) {
            db.up.remove(giver.id.to_string())?;
            db.down.remove(giver.id.to_string())?;
        }

        let (db_available, default_available) = match modifier {
            Karma::Up => (&db.up, DEFAULT_UP),
            Karma::Down => (&db.down, DEFAULT_DOWN),
        };

        let available_current = db_available.get_or(giver.id.to_string(), default_available)?;
        if available_current < 1 {
            let karma_giver_current = db.karma.get_or(giver.id.to_string(), 0)?;
            if karma_giver_current < 1 {
                bot.answer_callback_query(cq.id)
                    .text("not enough karma")
                    .await?;
                return Ok(());
            }

            let karma_giver = karma_giver_current - 1;
            db.karma.insert(giver.id.to_string(), karma_giver)?;
        } else {
            let available = available_current - 1;
            db_available.insert(giver.id.to_string(), available)?;
        }

        let karma_giver = db.karma.get_or(giver.id.to_string(), 0)?;

        let source = match available_current < 1 {
            true => "karma",
            false => "points",
        };

        db.karma.insert(receiver_id.to_string(), karma_receiver)?;

        bot.answer_callback_query(cq.id).text("thanks!").await?;

        if let Some(msg) = cq.message {
            let last_message_key = format!("{}-{}", msg.chat.id, receiver_id);
            if let Some(last_message) = db.last_message.get(&last_message_key)? {
                bot.delete_message(msg.chat.id, last_message).await.ok();
            }

            let receiver_chat = bot.get_chat(receiver_id).await?;
            let receiver_mention = mention_chat(&receiver_chat);

            let text = format!(
                "{} reputation of {} ({})\n\
                <i>thanks to {}'s {} ({})</i>",
                modifier,
                receiver_mention,
                karma_receiver,
                mention_user(&giver),
                source,
                karma_giver
            );

            bot.edit_message_text(msg.chat.id, msg.id, text).await?;
            db.last_message.insert(&last_message_key, msg.id)?;
        }
    }

    Ok(())
}

pub async fn callback_handler(
    bot: DefaultParseMode<Bot>,
    db: Arc<Store>,
    cq: CallbackQuery,
) -> ResponseResult<()> {
    match callback_handler_internal(bot, db, cq).await {
        Ok(_) => Ok(()),
        Err(err) => match err.downcast::<teloxide::RequestError>() {
            Ok(err) => Err(err),
            Err(err) => {
                log::error!("Generic error: {}", err);
                Ok(())
            }
        },
    }
}
