// Jest setup file for global mocks and configurations

// Mock console.log and console.error in tests
global.console = {
  ...console,
  log: jest.fn(),
  error: jest.fn(),
  warn: jest.fn(),
};

// Mock process.exit to prevent tests from actually exiting
global.process.exit = jest.fn();

// Add global test timeout
jest.setTimeout(10000);
