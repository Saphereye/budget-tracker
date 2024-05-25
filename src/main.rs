use chrono::{Local, NaiveDate};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::{collections::HashMap, env, process::Command};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Add entry
    #[arg(short, long)]
    add: bool,

    /// Edit entries
    #[arg(short, long)]
    edit: bool,
}

/// The `Expense` struct; helps reading/writing data in a structured manner. It reflects the schema of the database.
#[derive(Debug)]
struct Expense {
    date: String,
    description: String,
    expense_type: String,
    amount: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.add {
        add_expense()?;
    }

    if args.edit {
        edit_expenses()?;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let expenses = match read_csv("expenses.csv") {
        Ok(expenses) => expenses,
        Err(err) => {
            eprintln!("Error reading CSV: {}", err);
            match create_expenses_csv() {
                Ok(_) => Vec::new(),
                Err(err) => {
                    eprintln!("Error creating CSV: {}", err);
                    return Err(err);
                }
            }
        }
    };

    let mut should_quit = false;
    let mut table_state = TableState::default().with_selected(Some(0));
    while !should_quit {
        terminal.draw(|f| ui(f, &expenses, &mut table_state))?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}

/**
Function to add and expense to the database.

Takes input from `stdin` for date, description, expense type and amount.
Support YYYY-MM-DD and YYYY/MM/DD date format as input.
For amount no denoination is expected as of now.
*/
fn add_expense() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();

    let date = loop {
        print!("Enter date (YYYY-MM-DD or YYYY/MM/DD, leave empty for today's date): ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();

        if input.is_empty() {
            break Local::now().format("%Y-%m-%d").to_string();
        } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
            break date.to_string();
        } else if let Ok(date) = NaiveDate::parse_from_str(&input, "%Y/%m/%d") {
            break date.to_string();
        } else {
            println!(
                "Invalid date format. Please enter the date in YYYY-MM-DD or YYYY/MM/DD format."
            );
            input.clear();
        }
    };
    input.clear();

    print!("Enter description:");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    let description = input.trim().to_string();
    input.clear();

    print!("Enter expense type:");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    let expense_type = input.trim().to_string();
    input.clear();

    print!("Enter amount:");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    let amount: f64 = input.trim().parse()?;

    let expense = Expense {
        date,
        description,
        expense_type,
        amount,
    };

    append_to_csv("expenses.csv", &expense)?;
    println!("Added your data to the db!");

    Ok(())
}

/// Allows editing the database by specifying an EDITOR environment variable. By default its nano.
fn edit_expenses() -> Result<(), Box<dyn std::error::Error>> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let home_dir = dirs::home_dir().ok_or("Unable to determine user's home directory")?;
    let file_path = home_dir
        .join(".local")
        .join("share")
        .join("budget-tracker")
        .join("expenses.csv");

    Command::new(editor).arg(file_path).status()?;

    Ok(())
}

/// Allows adding data to the end of the database
fn append_to_csv(file_name: &str, expense: &Expense) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Unable to determine user's home directory");
            return Err("Unable to determine user's home directory".into());
        }
    };

    let file_path = home_dir
        .join(".local")
        .join("share")
        .join("budget-tracker")
        .join(file_name);
    let mut file = fs::OpenOptions::new().append(true).open(file_path)?;
    let data = format!(
        "{},{},{},{}\n",
        expense.date, expense.description, expense.expense_type, expense.amount
    );
    file.write_all(data.as_bytes())?;

    Ok(())
}

/// Read the database if its present from ~/.local/share/budget-tracker/expenses.csv;
/// if not present it returns an error.
fn read_csv(file_name: &str) -> Result<Vec<Expense>, Box<dyn std::error::Error>> {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Unable to determine user's home directory");
            return Err("Unable to determine user's home directory".into());
        }
    };

    let file_path = home_dir
        .join(".local")
        .join("share")
        .join("budget-tracker")
        .join(file_name);
    let file = fs::File::open(file_path)?;

    let reader = BufReader::new(file);
    let mut expenses = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if index == 0 {
            continue; // Skip header
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() == 4 {
            let expense = Expense {
                date: fields[0].to_string(),
                description: fields[1].to_string(),
                expense_type: fields[2].to_string(),
                amount: fields[3].parse::<f64>()?,
            };
            expenses.push(expense);
        }
    }
    Ok(expenses)
}

/// Creates the database. Usually called when running the program for the first time.
fn create_expenses_csv() -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Unable to determine user's home directory");
            return Err("Unable to determine user's home directory".into());
        }
    };

    let budget_tracker_dir = home_dir.join(".local").join("share").join("budget-tracker");
    if let Err(err) = fs::create_dir_all(&budget_tracker_dir) {
        eprintln!(
            "Error creating directory {}: {}",
            budget_tracker_dir.display(),
            err
        );
        return Err(err.into());
    }

    let expenses_file = budget_tracker_dir.join("expenses.csv");
    if let Err(err) = fs::File::create(&expenses_file) {
        eprintln!("Error creating file {}: {}", expenses_file.display(), err);
        return Err(err.into());
    }

    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, expenses: &[Expense], table_state: &mut TableState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(frame.size());

    // Split the second chunk (chunks[1]) vertically into two equal parts
    let charts_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let positive_chunk = charts_chunks[0];
    let negative_chunk = charts_chunks[1];

    // Calculate the total sum of amounts
    let total_amount: f64 = expenses.iter().map(|expense| expense.amount).sum();
    let total_spent: f64 = expenses
        .iter()
        .filter(|expense| expense.amount < 0.0)
        .map(|expense| expense.amount)
        .sum();
    let total_earned: f64 = expenses
        .iter()
        .filter(|expense| expense.amount >= 0.0)
        .map(|expense| expense.amount)
        .sum();

    // Expense Table
    let mut rows = expenses
        .iter()
        .map(|expense| {
            Row::new(vec![
                expense.date.clone(),
                expense.description.clone(),
                expense.expense_type.clone(),
                expense.amount.to_string(),
            ])
        })
        .collect::<Vec<Row>>();

    // Add the total amount row
    rows.push(
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Net Total Spent".to_string(),
            total_amount.to_string(),
        ])
        .style(Style::default().bold())
        .top_margin(1),
    );

    // Add the total debt row
    rows.push(
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Total Spent".to_string(),
            total_spent.to_string(),
        ])
        .style(Style::default().bold()),
    );

    // Add the total income row
    rows.push(
        Row::new(vec![
            "".to_string(),
            "".to_string(),
            "Total Earned".to_string(),
            total_earned.to_string(),
        ])
        .style(Style::default().bold()),
    );

    let widths = [
        Constraint::Length(15),
        Constraint::Length(55),
        Constraint::Length(30),
        Constraint::Length(10),
    ];

    let expense_table = Table::new(rows, widths)
        .block(Block::default().title("Transactions").borders(Borders::ALL))
        // .column_spacing(1)
        // .style(Style::default())
        .header(
            Row::new(vec!["Date", "Description", "Type", "Amount"]).style(Style::default().bold()),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>");

    frame.render_widget(expense_table, chunks[0]);
    // frame.render_stateful_widget(expense_table, chunks[0], table_state); // TODO: use this for TUI editing

    // Aggregate expenses by date
    let mut aggregated_expenses: HashMap<String, f64> = HashMap::new();
    for expense in expenses {
        let entry = aggregated_expenses
            .entry(expense.expense_type.clone())
            .or_insert(0.0);
        *entry += expense.amount;
    }

    // Separate positive and negative expenses
    let total_earned_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount >= 0.0)
        .collect();

    let total_spent_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount < 0.0)
        .map(|(date, amount)| (date, -amount))
        .collect();

    for (mut expense_data, chunk, title, color) in [
        (
            total_spent_data.clone(),
            positive_chunk,
            "Expenditure",
            Style::default().cyan(),
        ),
        (
            total_earned_data,
            negative_chunk,
            "Income",
            Style::default().red(),
        ),
    ] {
        // Convert expenses to chart data
        // Sort the expense data by date
        expense_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut labels = expense_data
            .iter()
            .map(|(date, _)| Span::raw(date.clone()))
            .collect::<Vec<Span>>();
        labels.insert(0, "".into());
        labels.push("".into());

        // Find the maximum expense amount
        let max_expense_amount = expense_data
            .iter()
            .map(|(_, amount)| *amount)
            .fold(f64::NEG_INFINITY, f64::max);

        // Convert type expenses to bar chart data
        let type_data: Vec<(&str, u64)> = expense_data
            .iter()
            .map(|(date, amount)| (date.as_str(), *amount as u64))
            .collect();

        let type_barchart = BarChart::default()
            .block(Block::default().title(title).borders(Borders::ALL))
            .bar_width(15)
            // .bar_gap(1)
            // .group_gap(3)
            .bar_style(color)
            .value_style(Style::default().white().bold())
            .label_style(Style::default().white())
            .data(&type_data)
            .max(max_expense_amount.ceil() as u64); // Set the maximum value to the next integer greater than the maximum expense amount

        frame.render_widget(type_barchart, chunk); // Render the type barchart
    }
}
