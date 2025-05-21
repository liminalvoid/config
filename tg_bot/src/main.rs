use dotenvy;
use std::error::Error;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    sugar::bot::BotMessagesExt,
    types::{
        ChatMemberKind, InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResultArticle,
        InputMessageContent, InputMessageContentText, Me, User,
    },
    utils::command::BotCommands,
};

/// These commands are supported:
#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text
    Help,
    /// Start
    Start,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    pretty_env_logger::init();
    log::info!("Starting dialogue bot...");

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_inline_query().endpoint(inline_query_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let actions = ["Новый конфиг", "Список конфигов"];

    for action in actions.chunks(1) {
        let row = action
            .iter()
            .map(|&version| InlineKeyboardButton::callback(version.to_owned(), version.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}

async fn is_user_has_access(bot: &Bot, user: &User) -> bool {
    let user_id = user.id;
    let chat_id = dotenvy::var("CHAT_ID").unwrap();

    let chat_member = bot.get_chat_member(chat_id, user_id).await.unwrap();

    match chat_member.kind {
        ChatMemberKind::Left | ChatMemberKind::Banned(_) | ChatMemberKind::Restricted(_) => {
            return false;
        }
        _ => return true,
    };
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let user = msg.from.as_ref().unwrap();

    let user_log_string = format!(
        "User {} (username: {}, ID: {})",
        user.full_name(),
        user.username.clone().unwrap_or("None".to_string()),
        user.id
    );

    log::info!("{} is trying to access bot", user_log_string);

    if !is_user_has_access(&bot, user).await {
        log::info!("{} has no access", user_log_string);

        return Ok(());
    }

    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                // Just send the description of all commands.
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Start) => {
                // Create a list of buttons and send them.
                let keyboard = make_keyboard();
                bot.send_message(msg.chat.id, "Выберите действие:")
                    .reply_markup(keyboard)
                    .await?;
            }

            Err(_) => {
                bot.send_message(msg.chat.id, "Команда не найдена").await?;
            }
        }
    }

    Ok(())
}

async fn inline_query_handler(
    bot: Bot,
    q: InlineQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let choose_action = InlineQueryResultArticle::new(
        "0",
        "Выберите действие",
        InputMessageContent::Text(InputMessageContentText::new("Выберите действие:")),
    )
    .reply_markup(make_keyboard());

    bot.answer_inline_query(q.id, vec![choose_action.into()])
        .await?;

    Ok(())
}

/// **IMPORTANT**: do not send privacy-sensitive data this way!!!
/// Anyone can read data stored in the callback button.
async fn callback_handler(bot: Bot, q: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref action) = q.data {
        let text = format!("You chose: {action}");

        // Tell telegram that we've seen this query, to remove 🕑 icons from the
        // clients. You could also use `answer_callback_query`'s optional
        // parameters to tweak what happens on the client side.
        bot.answer_callback_query(&q.id).await?;

        // Edit text of the message to which the buttons were attached
        if let Some(message) = q.regular_message() {
            bot.edit_text(message, text).await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_message_text_inline(id, text).await?;
        }

        log::info!("You chose: {}", action);
    }

    Ok(())
}
