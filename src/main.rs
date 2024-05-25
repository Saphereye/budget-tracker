use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;
use std::io;

mod expense;
use expense::*;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.add {
        Expense::add_expense()?;
    }

    if args.edit {
        Expense::edit_expenses("expenses.csv")?;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let expenses = match Expense::read_csv("expenses.csv") {
        Ok(expenses) => expenses,
        Err(err) => {
            eprintln!("Error reading CSV: {}", err);
            match Expense::create_expenses_csv() {
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

fn ui(frame: &mut Frame, expenses: &[Expense], _table_state: &mut TableState) {
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
                expense.expense_type.to_string(),
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
            .entry(expense.expense_type.to_string())
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
