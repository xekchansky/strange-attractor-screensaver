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

/// Capture request derived from the command line, if any.
enum CaptureRequest {
    /// Save one PNG after warm-up (fast-forwarded trail + settled camera).
    case screenshot(outputPath: String)
    /// Save a sequence of PNGs for GIF/APNG assembly.
    case record(outputDirectory: String, seconds: Double, frameStride: Int)
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
    /// Optional capture request (`--screenshot` / `--record`).
    let capture: CaptureRequest?
    /// Number of fast-forward integration steps before the scene starts
    /// (capture runs only). `nil` keeps the built-in default.
    let warmupSteps: Int?
    /// Snapshot resolution multiplier for capture runs. `nil` keeps the
    /// Retina-matching default of 2; recordings downscaled to a small
    /// animation can use 1 so the frame loop keeps up with the 60 Hz timer.
    let captureScale: Int?
    /// Starting camera orbit azimuth in degrees. `nil` picks a random angle;
    /// documentation captures pin it so a screenshot is reproducible.
    let cameraAzimuthDegrees: Double?
    /// Camera orbit elevation above the horizontal plane, in degrees. `nil`
    /// keeps the default π/5.
    let cameraElevationDegrees: Double?
    /// Explicit starting state for the integration. `nil` uses the
    /// selection's own start (a sampled or randomly offset point). Thomas is
    /// the reason this exists: its catalog midpoint start sits in the basin of
    /// a fixed point for most of the declared `b` range, so a pinned capture
    /// has to start well away from the origin to trace the attractor.
    let initialState: State?
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
          --screenshot <path>     save one PNG (after trail warm-up and camera
                                  settle) and exit; implies a fixed setup
          --record <dir>          save a sequence of PNG frames for GIF/APNG
                                  assembly and exit; implies a fixed setup
          --record-seconds <v>    duration of the capture in seconds (default 4)
          --frame-stride <n>      capture every n-th rendered frame (default 2)
          --warmup-steps <n>      integration steps fast-forwarded before the
                                  capture starts (default 60000; 0 records the
                                  trail growing from the random start)
          --capture-scale <n>     snapshot resolution multiplier (default 2;
                                  1 shrinks the pixel count 4x, which helps
                                  the frame loop keep a steady 60 Hz)
          --cam-azimuth <deg>     start the camera orbit at this azimuth
                                  (default: random)
          --cam-elevation <deg>   camera height above the horizontal plane
                                  (default: 36 degrees)
          --initial-state x,y,z   start the integration at this point instead
                                  of the selection's sampled start
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
        var capture: CaptureRequest?
        var warmup: Int?
        var captureScale: Int?
        var cameraAzimuth: Double?
        var cameraElevation: Double?
        var initialState: State?
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

        /// Parse an `x,y,z` triple into a ``State``.
        func state(after flag: String) throws -> State {
            let raw = try value(after: flag)
            let parts = raw.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            guard parts.count == 3 else {
                throw CommandLineArgumentsError.invalidParameter("\(flag) \(raw)")
            }
            let numbers = parts.compactMap(Double.init)
            guard numbers.count == 3, numbers.allSatisfy(\.isFinite) else {
                throw CommandLineArgumentsError.invalidParameter("\(flag) \(raw)")
            }
            return State(x: numbers[0], y: numbers[1], z: numbers[2])
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
            case "--screenshot":
                guard capture == nil else {
                    throw CommandLineArgumentsError.unknownArgument(
                        "\(argument): --screenshot and --record are mutually exclusive"
                    )
                }
                capture = .screenshot(outputPath: try value(after: argument))
            case "--record":
                guard capture == nil else {
                    throw CommandLineArgumentsError.unknownArgument(
                        "\(argument): --screenshot and --record are mutually exclusive"
                    )
                }
                capture = .record(outputDirectory: try value(after: argument), seconds: 4, frameStride: 2)
            case "--record-seconds":
                let seconds = try double(after: argument)
                guard seconds > 0 else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(seconds)")
                }
                switch capture {
                case .record(let directory, _, let stride):
                    capture = .record(outputDirectory: directory, seconds: seconds, frameStride: stride)
                case .screenshot, nil:
                    throw CommandLineArgumentsError.unknownArgument("\(argument) requires --record")
                }
            case "--frame-stride":
                let raw = try value(after: argument)
                guard let stride = Int(raw), stride >= 1 else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                switch capture {
                case .record(let directory, let seconds, _):
                    capture = .record(outputDirectory: directory, seconds: seconds, frameStride: stride)
                case .screenshot, nil:
                    throw CommandLineArgumentsError.unknownArgument("\(argument) requires --record")
                }
            case "-h", "--help":
                showHelp = true
            case "--warmup-steps":
                let raw = try value(after: argument)
                guard let steps = Int(raw), steps >= 0 else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                warmup = steps
            case "--capture-scale":
                let raw = try value(after: argument)
                guard let scale = Int(raw), (1...4).contains(scale) else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                captureScale = scale
            case "--cam-azimuth":
                let raw = try value(after: argument)
                guard let degrees = Double(raw), degrees.isFinite else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                cameraAzimuth = degrees
            case "--cam-elevation":
                let raw = try value(after: argument)
                guard let degrees = Double(raw), degrees.isFinite,
                      (-89...89).contains(degrees) else {
                    throw CommandLineArgumentsError.invalidParameter("\(argument) \(raw)")
                }
                cameraElevation = degrees
            case "--initial-state":
                initialState = try state(after: argument)
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
            capture: capture,
            warmupSteps: warmup,
            captureScale: captureScale,
            cameraAzimuthDegrees: cameraAzimuth,
            cameraElevationDegrees: cameraElevation,
            initialState: initialState,
            showHelp: showHelp
        )
    }
}
