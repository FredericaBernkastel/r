Monitor Reddit posts in real time, and notify via email.  

#### Example usage:
- Display all new posts in console:   
  `cargo +nightly run`  
  &nbsp;
- Filter by a specific subreddit:  
  `cargo +nightly run -- -s "AskReddit"`  
  &nbsp;
- Also filter by post title using regular expression:  
  `cargo +nightly run -- -s "AskReddit" -r "(?i)what"`  
  &nbsp;
- Also notify via email:    
  `cargo +nightly run -- -s "AskReddit" -r "(?i)what" --notify-email="email@example.com`  
  &nbsp; 
- `cargo +nightly run -- --help` for more help.

Before enabling email notifications, make sure correct SMTP relay credentials are provided in `./smtp_config.toml`