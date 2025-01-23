use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{collections::HashMap, io, thread::current};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

enum AgentState {
    Idle,
    Thinking,
    Working,
}

struct AgentConfig {
    host: String,
}

struct Agent {
    state: AgentState,
    config: AgentConfig,
}

impl Agent {
    fn new() -> Self {
        Self {
            state: AgentState::Idle,
            config: AgentConfig {
                host: String::from("127.0.0.1"),
            },
        }
    }

    fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }

    fn get_state(&self) -> &AgentState {
        &self.state
    }

    fn set_host(&mut self, host: String) {
        self.config.host = host;
    }

    fn get_host(&self) -> &String {
        &self.config.host
    }
}

static mut current_command: String = String::new();
static mut current_command_args: Vec<String> = vec![];

fn main() -> Result<(), io::Error> {
    let mut agent = Agent::new();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    
    let mut input = String::new();
    let commands = setup_commands();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(60), Constraint::Percentage(30), Constraint::Percentage(10)].as_ref())
                .split(size);

            // Display command output
            capture_command(&input, &commands);
            let output_widget = Paragraph::new("Foo")
                .block(Block::default().borders(Borders::ALL).title("Output"));
            f.render_widget(output_widget, chunks[0]);

            let status = format!("Target host: {}", agent.get_host());
            let status_widget = Paragraph::new(status)
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(status_widget, chunks[1]);
            // Input area
            let input_widget = Paragraph::new(input.as_ref() as &str)
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_widget, chunks[2]);
        })?;

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Enter => {
                        // Execute command on Enter key

                        unsafe {
                            match current_command.as_str() {
                                "sethost" => agent.set_host(current_command_args[0].clone()),
                                "exit" => break,
                                _ => {}
                            }
                        }

                        input.clear();  // Cleaesponser input after execution
                    }
                    crossterm::event::KeyCode::Esc => break,  // Exit on Esc key
                    crossterm::event::KeyCode::Char(c) => {
                        input.push(c);  // Add character to input
                    }
                    crossterm::event::KeyCode::Backspace => {
                        input.pop();  // Remove last character on Backspace
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn setup_commands() -> HashMap<String, String> {
    let mut commands = HashMap::new();
    commands.insert("sethost".to_string(), "Set host".to_string());
    commands.insert("help".to_string(), "Available commands: hello, quit, sethost".to_string());
    commands.insert("quit".to_string(), "Goodbye!".to_string());
    commands
}

fn capture_command(input: &str, commands: &HashMap<String, String>) {
    if input.is_empty() {
        return;
    }

    let cmd = input.split(" ").collect::<Vec<&str>>()[0];
    let args = input.split(" ").collect::<Vec<&str>>()[1..].join(" ");

    unsafe {
        current_command = String::from(cmd);
        current_command_args = args.split(" ").map(|s| s.to_string()).collect();
    }
}