use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{prelude::*, widgets::*};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Date of transaction
    #[arg(short, long)]
    date: Option<String>,

    /// Description of transaction
    #[arg(short, long)]
    description: Option<String>,

    /// Type of transaction
    #[arg(short, long)]
    expense_type: Option<String>,

    /// Amount of transaction
    #[arg(short, long)]
    amount: Option<f64>,
}

#[derive(Debug)]
struct Expense {
    date: String,
    description: String,
    expense_type: String,
    amount: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if Some(date), Some(desc)

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let expenses = match read_csv("expenses.csv") {
        Ok(expenses) => {
            expenses
        }
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
    while !should_quit {
        terminal.draw(|f| ui(f, &expenses))?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}

fn append_to_csv(file_name: &str, expense: &Expense) -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Unable to determine user's home directory");
            return Err("Unable to determine user's home directory".into());
        }
    };

    let file_path = home_dir.join(".local").join("share").join("budget-tracker").join(file_name);
    let mut file = fs::OpenOptions::new().append(true).open(file_path)?;

    writeln!(
        file,
        "{},{},{},{}",
        expense.date, expense.description, expense.expense_type, expense.amount
    )?;

    Ok(())
}

fn read_csv(file_name: &str) -> Result<Vec<Expense>, Box<dyn std::error::Error>> {
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            eprintln!("Unable to determine user's home directory");
            return Err("Unable to determine user's home directory".into());
        }
    };

    let file_path = home_dir.join(".local").join("share").join("budget-tracker").join(file_name);
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

fn ui(frame: &mut Frame, expenses: &[Expense]) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(frame.size());

    // Split the second chunk (chunks[1]) vertically into two equal parts
    let charts_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let positive_chunk = charts_chunks[0];
    let negative_chunk = charts_chunks[1];

    // Expense Table
    let rows = expenses
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

    let widths = [
        Constraint::Length(15),
        Constraint::Length(30),
        Constraint::Length(20),
        Constraint::Length(15),
    ];

    let expense_table = Table::new(rows, widths)
        .block(Block::default().title("All Expenses").borders(Borders::ALL))
        .column_spacing(1)
        .style(Style::default().blue())
        .header(
            Row::new(vec!["Date", "Description", "Expense Type", "Amount"])
                .style(Style::default().bold())
                .bottom_margin(1),
        )
        .block(Block::default().title("Expenses Table"))
        .highlight_style(Style::new().reversed())
        .highlight_symbol(">>");

    frame.render_widget(expense_table, chunks[0]); // Render the expense table on the left

    // Aggregate expenses by date
    let mut aggregated_expenses: HashMap<String, f64> = HashMap::new();
    for expense in expenses {
        let entry = aggregated_expenses
            .entry(expense.expense_type.clone())
            .or_insert(0.0);
        *entry += expense.amount;
    }

    // Separate positive and negative expenses
    let positive_expenses_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount >= 0.0)
        .collect();

    let negative_expenses_data: Vec<(String, f64)> = aggregated_expenses
        .clone()
        .into_iter()
        .filter(|(_, amount)| *amount < 0.0)
        .map(|(date, amount)| (date, -amount))
        .collect();

    for (mut expense_data, chunk, title, color) in [
        (
            positive_expenses_data.clone(),
            positive_chunk,
            "Expenses",
            Style::default().cyan(),
        ),
        (
            negative_expenses_data,
            negative_chunk,
            "Profits",
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
            .bar_width(10)
            .bar_gap(1)
            .group_gap(3)
            .bar_style(color)
            .value_style(Style::default().white().bold())
            .label_style(Style::default().white())
            .data(&type_data)
            .max(max_expense_amount.ceil() as u64); // Set the maximum value to the next integer greater than the maximum expense amount

        frame.render_widget(type_barchart, chunk); // Render the type barchart
    }
}
