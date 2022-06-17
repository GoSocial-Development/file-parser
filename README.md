# FILE PARSER

## Setup and running
 1. `rustc main.rs`
 2. Create a `config.ini` file - look at `config.ini.example` for more info.
 3. Create a `input` folder and drop in some files in there.
 4. Run the executable.
 5. Check the `errors.txt` file or the `output` folder.

## INI Configuration
Consists of 4 sections:
1. `CONFIG`:
    - `INPUT_FOLDER` - folder where the files are
    - `ROW_TYPES` - types of row to expect, comma separated
    ```
    ROW_TYPES=A,B,C 
    ```
2. `ROW_CONFIG`:
    - Each row type must be configured with the fields it contains; Fields are separated by a `+`. Supported fields types are `string`, `number` and `date`. The `date` field requires a format string that can be passed using `|` like ```date|format (date|%m-%d-%Y)```. Date format options can be found [here](https://docs.rs/chrono/0.3.1/chrono/format/strftime/index.html)
    - Field can be marked as optional with a `*`, a field marked with `*` will not throw an error when the value is empty. 
    ```
    A=string*+string+number+date|%m/%d/%Y
    ```

3. `COLUMN_NAMES`:
    - Name of the sql table columns to be used when building the sql query. 
     ```
    A=Address+Phone
    ```
    
4. `COLUMN_LINKS`:
    - Add special columns, their value will be taken from other rows. Composed of the name of the column, followed by a ``->`` followed by the row type from where the value should be taken, followed by ``:``, followed by the number of column from where the value is taken.
     ```
    A=ColumnName->B:1+AnotherColumn->C:0
    ```
    
5. `TABLE_NAMES`:
    - Specify the table name to be used in queries for a specific type of row.
     ```
    A=MyTable
    B=MyOtherTable
    ```
## Output
### Errors:
All errors are outputed to `errors.txt`.
### SQL FILES:
Are outputed to the `output` folder, a `.sql` file for each row type.

## Notes
- No sql files are outputed if errors are detected.
- `H` has hardocoded two fields `hdrFileNamem` (stores the file name),`hdrFileQuarter` (stores the last 4 chars of the file name, excluding the extension).