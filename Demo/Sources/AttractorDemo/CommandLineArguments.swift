import Foundation
import AttractorCore

/// Error thrown while parsing command-line arguments.
enum CommandLineArgumentsError: Error, Equatable {
    case unknownArgument(String)
    case missingValue(for: String)
    case invalidParameter(String)
}

/// Optional overrides for the simulation hyperparameters, applied on top of
/// whatever the selection (manual or random) chose.
struct SimulationOverrides {
    /// Override for the trail length (number of points kept).
    var trailLength: Int?
    /// Override for the simulated-time multiplier per rendered frame.
    var simulationSpeed: Double?
    /// Override for the integration step size (accuracy).
    var stepSize: Double?
    /// Override for the camera orbit angular velocity (radians/second).
    var cameraRotationSpeed: Double?
    /// Override for the trail color gradient advance speed.
    var trailColorSpeed: Double?
    /// Override for the trail color mode.
    var colorMode: TraceColorMode?
}

/// Parsed command-line options for the demo app.
struct CommandLineArguments {
    /// Fixed attractor id, when `--attractor` was supplied.
    let attractorID: String?
    /// Raw `name=value` parameters. Values are kept as strings — numeric
    /// conversion and name validation happen against the catalog in `main`,
    /// which knows which parameters each attractor actually declares.
    let parameters: [String: String]
    /// Optional simulation hyperparameter overrides.
    let overrides: SimulationOverrides
    /// True when `--help` was requested.
    let showHelp: Bool

    static let usage = """
    Usage: AttractorDemo [options]

    Runs the attractor screensaver simulation in a SceneKit window.

    Options:
      -a, --attractor <id>        fix a specific attractor (lorenz, rossler, ...)
      -p, --param name=val        pin a simulation parameter value (repeatable)
          --trail-length <n>      override trail length (number of points)
          --speed <v>             override simulation speed multiplier
          --step <v>              override integration step size
          --cam-speed <v>         override camera orbit speed (rad/s)
          --trail-color-speed <v> override trail color gradient speed
          --color-mode <mode>     override color mode: trail|age|depth|spatial
      -h, --help                  show this help (including available attractors) and exit

    Bare `name=value` tokens are also treated as pinned parameters, e.g.
      AttractorDemo --attractor rossler a=0.2 b=0.2 c=5.7

    Without options, a random attractor with random parameters is chosen.
    """

    /// Parse command-line arguments (excluding the executable path).
    /// - Parameter arguments: `CommandLine.arguments` as passed to the app.
    /// - Throws: ``CommandLineArgumentsError`` on an unknown flag, a missing
    ///   value or a broken `name=value`/number token.
    static func parse(_ arguments: [String]) throws -> CommandLineArguments {
        var attractorID: String?
        var parameters: [String: String] = [:]
        var overrides = SimulationOverrides()
        var showHelp = false
        var index = 1 // skip the executable path

        func value(after flag: String) throws -> String {
            guard index + 1 < arguments.count else {
                throw CommandLineArgumentsError.missingValue(for: flag)
            }
            index += 1
            return arguments[index]
        }

        func double(after flag: String) throws -> Double {
            let raw = try value(after: flag)
            guard let number = Double(raw) else {
                throw CommandLineArgumentsError.invalidParameter("\(flag) \(raw)")
            }
            return number
        }

        func addParameter(_ raw: String) throws {
            guard let eq = raw.firstIndex(of: "="), eq != raw.startIndex else {
                throw CommandLineArgumentsError.invalidParameter(raw)
            }
            parameters[String(raw[..<eq])] = String(raw[raw.index(after: eq)...])
        }

        while index < arguments.count {
            let argument = arguments[index]
            switch argument {
            case "-a", "--attractor":
                attractorID = try value(after: argument)
            case "-p", "--param":
                try addParameter(try value(after: argument))
            case "--trail-length":
                let raw = try value(after: argument)
                guard let number = Int(raw), number >= 1 else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                overrides.trailLength = number
            case "--speed":
                overrides.simulationSpeed = try double(after: argument)
            case "--step":
                overrides.stepSize = try double(after: argument)
            case "--cam-speed":
                overrides.cameraRotationSpeed = try double(after: argument)
            case "--trail-color-speed":
                overrides.trailColorSpeed = try double(after: argument)
            case "--color-mode":
                let raw = try value(after: argument)
                guard let mode = TraceColorMode(rawValue: raw) else {
                    throw CommandLineArgumentsError.invalidParameter("\"\(argument) \(raw)\"")
                }
                overrides.colorMode = mode
            case "-h", "--help":
                showHelp = true
            default:
                // Any bare `name=value` token is treated as a parameter.
                if argument.contains("=") {
                    try addParameter(argument)
                } else {
                    throw CommandLineArgumentsError.unknownArgument(argument)
                }
            }
            index += 1
        }

        return CommandLineArguments(
            attractorID: attractorID,
            parameters: parameters,
            overrides: overrides,
            showHelp: showHelp
        )
    }
}
