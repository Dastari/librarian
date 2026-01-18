/**
 * Logger utility that respects environment settings.
 * 
 * In development: All logs are shown
 * In production: Only warnings and errors are shown
 * 
 * Usage:
 *   import { logger } from '../lib/logger'
 *   logger.log('Debug info:', data)
 *   logger.warn('Warning:', message)
 *   logger.error('Error:', error)
 *   logger.debug('Verbose debug info:', data)
 */

const isDev = import.meta.env.DEV

type LogLevel = 'debug' | 'log' | 'info' | 'warn' | 'error'

interface LoggerConfig {
  /** Minimum level to log (debug < log < info < warn < error) */
  minLevel: LogLevel
  /** Whether to include timestamps in logs */
  timestamps: boolean
  /** Prefix for all log messages */
  prefix: string
}

const LOG_LEVELS: Record<LogLevel, number> = {
  debug: 0,
  log: 1,
  info: 2,
  warn: 3,
  error: 4,
}

const defaultConfig: LoggerConfig = {
  minLevel: isDev ? 'debug' : 'warn',
  timestamps: isDev,
  prefix: '[Librarian]',
}

function shouldLog(level: LogLevel): boolean {
  return LOG_LEVELS[level] >= LOG_LEVELS[defaultConfig.minLevel]
}

function formatArgs(level: LogLevel, args: unknown[]): unknown[] {
  const parts: unknown[] = []
  
  if (defaultConfig.prefix) {
    parts.push(defaultConfig.prefix)
  }
  
  if (defaultConfig.timestamps) {
    parts.push(`[${new Date().toISOString()}]`)
  }
  
  parts.push(`[${level.toUpperCase()}]`)
  
  return [...parts, ...args]
}

/**
 * Development-aware logger.
 * Use this instead of console.log/warn/error for cleaner production builds.
 */
export const logger = {
  /**
   * Debug-level logging. Only shown in development.
   * Use for verbose debugging information.
   */
  debug: (...args: unknown[]): void => {
    if (shouldLog('debug')) {
      console.debug(...formatArgs('debug', args))
    }
  },

  /**
   * General logging. Only shown in development.
   * Use for informational messages during development.
   */
  log: (...args: unknown[]): void => {
    if (shouldLog('log')) {
      console.log(...formatArgs('log', args))
    }
  },

  /**
   * Info-level logging. Only shown in development.
   * Use for notable events that aren't errors.
   */
  info: (...args: unknown[]): void => {
    if (shouldLog('info')) {
      console.info(...formatArgs('info', args))
    }
  },

  /**
   * Warning-level logging. Always shown.
   * Use for potential issues that don't prevent operation.
   */
  warn: (...args: unknown[]): void => {
    if (shouldLog('warn')) {
      console.warn(...formatArgs('warn', args))
    }
  },

  /**
   * Error-level logging. Always shown.
   * Use for errors that need attention.
   */
  error: (...args: unknown[]): void => {
    if (shouldLog('error')) {
      console.error(...formatArgs('error', args))
    }
  },

  /**
   * Group related logs together (development only).
   */
  group: (label: string): void => {
    if (isDev) {
      console.group(`${defaultConfig.prefix} ${label}`)
    }
  },

  /**
   * End a log group.
   */
  groupEnd: (): void => {
    if (isDev) {
      console.groupEnd()
    }
  },

  /**
   * Log a table (development only).
   */
  table: (data: unknown): void => {
    if (isDev) {
      console.table(data)
    }
  },

  /**
   * Time an operation (development only).
   */
  time: (label: string): void => {
    if (isDev) {
      console.time(`${defaultConfig.prefix} ${label}`)
    }
  },

  /**
   * End timing an operation.
   */
  timeEnd: (label: string): void => {
    if (isDev) {
      console.timeEnd(`${defaultConfig.prefix} ${label}`)
    }
  },
}

/**
 * Create a scoped logger with a custom prefix.
 * Useful for per-component or per-module logging.
 * 
 * @example
 * const log = createLogger('TorrentService')
 * log.info('Starting download...') // [Librarian:TorrentService] [INFO] Starting download...
 */
export function createLogger(scope: string) {
  const scopedPrefix = `${defaultConfig.prefix}:${scope}`
  
  return {
    debug: (...args: unknown[]): void => {
      if (shouldLog('debug')) {
        console.debug(scopedPrefix, ...args)
      }
    },
    log: (...args: unknown[]): void => {
      if (shouldLog('log')) {
        console.log(scopedPrefix, ...args)
      }
    },
    info: (...args: unknown[]): void => {
      if (shouldLog('info')) {
        console.info(scopedPrefix, ...args)
      }
    },
    warn: (...args: unknown[]): void => {
      if (shouldLog('warn')) {
        console.warn(scopedPrefix, ...args)
      }
    },
    error: (...args: unknown[]): void => {
      if (shouldLog('error')) {
        console.error(scopedPrefix, ...args)
      }
    },
  }
}
