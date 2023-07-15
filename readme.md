Monitor new posts in a single subreddit, filter post titles using a regular expression, and notify via email.  
- Example usage (print in console):   
`cargo +nightly run -- -s "AskReddit" -r "(?i)what"`
- Example usage (also notify via email):  
- `cargo +nightly run -- -s "AskReddit" -r "(?i)what" --notify-email="email@example.com`
- `cargo +nightly run -- --help` for more help.

Before enabling email, make sure correct SMTP relay credentials are provided in `./smtp_config.toml`