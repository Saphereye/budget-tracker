# Budget Tracker ![crates.io](https://img.shields.io/crates/v/budget-tracker.svg)

## Description
A minimal TUI based budget tracker.

Users can track date, a brief description, the type of purchase and the total amount spent/received.
For example, the following are the types of purchases the users can follow, although users are allowed to put any type they want.
- Food
- Gifts
- Health/medical
- Home
- Transportation
- Personal
- Pets
- Utilities
- Travel
- Debt

The data is stored at `~/.local/share/budget-tracker/expenses.csv`.

## Usage
- To install the program, make sure to have cargo [installed](https://doc.rust-lang.org/cargo/getting-started/installation.html), then run the following command.

```bash
cargo install budget-tracker
```

If the PATH is not set directly add the following to your shell profile.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

- To see graphical data run:
```bash
budget-tracker
```

- To add a new entry (add `-` infront of amount if you received money) run:
```bash
budget-tracker -a
```

## Screenshot
![](https://github.com/Saphereye/budget-tracker/blob/main/assets/image.png)
