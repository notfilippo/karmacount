use teloxide::types::{Chat, User, UserId};

pub mod group_command;
pub mod message;
pub mod root_command;
pub mod user_command;

const PRIVACY_NAME: &str = "??? (Privacy settings)";

pub(crate) fn mention_chat(chat: &Chat) -> String {
    let receiver_name = chat
        .username()
        .map(|username| format!("@{}", username))
        .unwrap_or_else(|| chat.first_name().unwrap_or(PRIVACY_NAME).to_string());
    format!("<a href=\"tg://user?id={}\">{}</a>", chat.id, receiver_name)
}

pub(crate) fn mention_id(id: &UserId) -> String {
    format!("<a href=\"tg://user?id={}\">{}</a>", id, PRIVACY_NAME)
}

pub(crate) fn mention_user(user: &User) -> String {
    format!(
        "<a href=\"tg://user?id={}\">{}</a>",
        user.id,
        user.username
            .clone()
            .map(|username| format!("@{}", username))
            .unwrap_or_else(|| user.full_name())
    )
}
