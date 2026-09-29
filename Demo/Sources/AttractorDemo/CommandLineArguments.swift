import Foundation

/// Error thrown while parsing command-line arguments.
enum CommandLineArgumentsError: Error, Equatable {
    case unknownArgument(String)
    case missingValue(for: String)
    case invalidParameter(String)
}

/// Parsed command-line options for the demo app.
struct CommandLineArguments {
    /// Fixed attractor id, when `--attractor` was supplied.
    let attractorID: String?
    /// Raw `name=value` parameters. Values are kept as strings — numeric
    /// conversion and name validation happen against the catalog in `main`,
    /// which knows which parameters each attractor actually declares.
    let parameters: [String: String]
    /// True when `--help` was requested.
    let showHelp: Bool

    static let usage = """
    Usage: AttractorDemo [options]

    Runs the attractor screensaver simulation in a SceneKit window.

    Options:
      -a, --attractor <id>  fix a specific attractor (lorenz, rossler, thomas, ...)
      -p, --param name=val  pin a simulation parameter value (repeatable)
      -h, --help            show this help (including available attractors) and exit

    Bare `name=value` tokens are also treated as pinned parameters, e.g.
      AttractorDemo --attractor rossler a=0.2 b=0.2 c=5.7

    Without options, a random attractor with random parameters is chosen.
    """

    /// Parse command-line arguments (excluding the executable path).
    /// - Parameter arguments: `CommandLine.arguments` as passed to the app.
    /// - Throws: ``CommandLineArgumentsError`` on an unknown flag or a broken
    ///   `name=value` token.
    static func parse(_ arguments: [String]) throws -> CommandLineArguments {
        var attractorID: String?
        var parameters: [String: String] = [:]
        var showHelp = false
        var index = 1 // skip the executable path

        func value(after flag: String) throws -> String {
            guard index + 1 < arguments.count else {
                throw CommandLineArgumentsError.missingValue(for: flag)
            }
            index += 1
            return arguments[index]
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
            showHelp: showHelp
        )
    }
}
