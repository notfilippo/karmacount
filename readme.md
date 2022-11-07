# Karma Count bot

## What is this?

@karmacountbot is a telegram bot that keeps track of karma points in a group
chat. Replying to a message of another user with "+" or "-", optionally followed
by arbitrary text, increments or decrements the karma of that user. Assignable
karma points are restored each day at midnight UTC.

You can assign 6 "+" points and 2 "-" points.

## How to use it?

Just add @karmacountbot to a group chat and start using it.

## How to run it?

### Requirements

- Rust 1.54.0 or later
- A Telegram bot token

### Running

```bash
$ export TOKEN=your_token
$ export ROOT=your_user_id
$ cargo run
```

The bot will create a `data` folder in the current working directory. This
folder contains the k-v store used to persist karma points across reboots.

## License

This project is licensed under the terms of the MIT license.
