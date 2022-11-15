use teloxide::types::Chat;

pub mod group_command;
pub mod message;
pub mod root_command;
pub mod user_command;

pub(crate) fn mention_chat(chat: &Chat) -> String {
    let receiver_name = chat
        .username()
        .map(|username| format!("@{}", username))
        .unwrap_or(chat.first_name().unwrap_or("N/A").to_string());
    format!("<a href=\"tg://user?id={}\">{}</a>", chat.id, receiver_name)
}
