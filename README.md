# FILE PARSER

## Setup and running the bot
 1. `rustc main.rs`
 2. Create a `config.ini` file - look at `config.ini.example` for more info

## INI Configuration
Consists of two sections:
1. `ROW_CONFIG`:
    - Each row type must be configured with the fields it contains; Fields are separated by a `+`. Supported fields types are `string`, `number` and `date`. The `date` field requires a format string that can be passed using `|` like ```date|format (date|%m-%d-%Y)```. Date format options can be found [here](https://docs.rs/chrono/0.3.1/chrono/format/strftime/index.html)
    - Field can be marked as optional with a `*`, a field marked with `*` will not throw an error when the value is empty. 

    ```
    A=string*+string+number+date|%m/%d/%Y
    ```
2. `CONFIG`:
    - `INPUT_FOLDER` - folder where the files are
    - `IGNORE_EMPTY_LINES = 1` - ignore empty lines

## Output
All errors are outputed to `errors.txt`.