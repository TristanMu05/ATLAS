# ATLAS USE SCENARIOS

## Scenario 1: Telemetry Monitoring
- **Description**: The system is in NORMAL mode, collecting telemetry data from the satellite and transmitting it to the ground station. The ground station receives and processes the data, displaying it on a dashboard for real-time monitoring.
- **Expected Outcome**: The ground station successfully receives and displays telemetry data, allowing operators to monitor the satellite's status and performance.

## Scenario 2: Command Execution
- **Description**: The ground station sends a command to the satellite to perform a specific action (e.g., adjust orientation, activate a sensor). The satellite receives the command, validates it, and executes the requested action.
- **Expected Outcome**: The satellite successfully executes the command, and the ground station receives confirmation of the action taken.

## Scenario 3: Safe Mode Activation
- **Description**: The satellite detects a critical fault (e.g., low battery voltage) and automatically transitions to SAFE mode to protect itself. The ground station receives an alert about the fault and the mode change.
- **Expected Outcome**: The satellite successfully transitions to SAFE mode, and the ground station receives the alert, allowing operators to take appropriate actions.

## Scenario 4: Diagnostic Mode Testing
- **Description**: The satellite enters DIAGNOSTIC mode to perform self-tests and system checks. The ground station receives diagnostic data and logs the results for analysis.
- **Expected Outcome**: The satellite successfully performs diagnostics, and the ground station receives and logs the diagnostic data for further analysis.

## Scenario 5: Telemetry Replay
- **Description**: The ground station uses previously recorded telemetry data to simulate a replay of the satellite's behavior. This allows operators to analyze past events and test responses without needing live data.
- **Expected Outcome**: The ground station successfully replays the telemetry data, allowing operators to analyze past events and test responses effectively.

