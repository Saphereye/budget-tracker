# Budget Tracker ![crates.io](https://img.shields.io/crates/v/budget-tracker.svg)

## Description
A minimal TUI based budget tracker.

Track your expenses and income by recording the date, a brief description, the type of transaction, and the amount spent or received. You can create custom expense types when adding transactions. For example, you might use:
- Food
- Travel
- Fun
- Medical
- Personal

The data by default is stored at `~/.local/share/budget-tracker/expenses.csv`.

## Usage
- To install the program, make sure to have [cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html), then run the following command.

```bash
cargo install budget-tracker
```

If the `PATH` is not set directly add the following to your shell profile.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

- To see graphical representation of your budget, run:
```bash
budget-tracker
```

- To add a new entry (add `-` infront of amount to show expenses) run any one of the following:
```bash
budget-tracker --add
budget-tracker -a
```

- To manually edit the database run any one of:
```bash
budget-tracker --edit
budget-tracker -e
```

By default it opens using `nano`. To specify an editor set the `EDITOR` environment variable.
```bash
EDITOR=vim budget-tracker --edit
```

This will open the file in vim.

- To search for a keyword or a particular expense type you can run as follows
```
budget-tracker -s <SEARCH_QUERY>
budget-tracker --search <SEARCH_QUERY>
```

Here the search query can either be a substring of the description (the search support fuzzy searching) or the expense type, the program automatically accounts for both.

- To exit press 'q'

## Screenshot
![](https://raw.githubusercontent.com/Saphereye/budget-tracker/main/assets/image.png)
