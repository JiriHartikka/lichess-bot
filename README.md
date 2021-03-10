# Lichess Bot
A bot implementation for lichess.org.

## Prerequisites
You need an actived Lichess bot account and a personal access token to use this bot. First, create a new lichess.org
account. Only a new account with no games played can be converted to bot account.

After generating the account, navigate to Lichess -> Settings -> API Access Tokens. Generate new personal
access token with permissions for profile and bot play.

After creating an account and generating the access token, you must make a http-request to convert 
the account to bot account, see: https://lichess.org/api#operation/botAccountUpgrade.

## Running
After activating your bot account and generating a personal access token, the bot can be started with
`LICHESS_ACCESS_TOKEN=<my-lichess-bot-account-access-token> cargo run --release`
where you should replace "<my-lichess-bot-account-access-token>" with your personal access token. 

## Notes
* Package `libssl-dev` needs to be manually installed on Ubuntu for the reqwest package to work. 