module.exports = {
  red: jest.fn((text) => `red:${text}`),
  green: jest.fn((text) => `green:${text}`),
  yellow: jest.fn((text) => `yellow:${text}`),
  cyan: jest.fn((text) => `cyan:${text}`),
  magenta: jest.fn((text) => `magenta:${text}`),
};
