import os from 'os';
import path from 'path';
import { readFileSync, writeFileSync, existsSync } from 'fs';

const CONFIG_FILENAME = '.formanatorrc.json';
const CONFIG_PATH = path.join(os.homedir(), CONFIG_FILENAME);

interface IntegrationAuth {
  cookies: Record<string, string>;
  lastUpdated: string;
}

interface Config {
  accessToken: string;
  email?: string;
  integrations?: Record<string, IntegrationAuth>;
}

export const maybeGetAccessToken = (
  maybeAccessToken: string | null | undefined,
): string | null => {
  if (maybeAccessToken) return maybeAccessToken;
  return getAccessToken();
};

export const getAccessToken = (): string | null => {
  if (!existsSync(CONFIG_PATH)) {
    return null;
  }

  const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
  const parsedConfig = JSON.parse(rawConfig) as Config;

  return parsedConfig.accessToken;
};

export const getEmail = (): string | undefined => {
  if (!existsSync(CONFIG_PATH)) {
    return undefined;
  }

  const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
  const parsedConfig = JSON.parse(rawConfig) as Config;

  return parsedConfig.email;
};

const readConfig = (): Config | null => {
  if (!existsSync(CONFIG_PATH)) {
    return null;
  }
  try {
    const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
    return JSON.parse(rawConfig) as Config;
  } catch {
    return null;
  }
};

const writeConfig = (config: Config): void => {
  writeFileSync(CONFIG_PATH, JSON.stringify(config, null, 2));
};

export const storeConfig = ({ accessToken, email }: { accessToken: string; email?: string }): void => {
  const existing = readConfig();
  writeConfig({ ...existing, accessToken, email });
};

export const getIntegrationAuth = (
  providerName: string,
): IntegrationAuth | null => {
  const config = readConfig();
  return config?.integrations?.[providerName] ?? null;
};

export const storeIntegrationAuth = (
  providerName: string,
  cookies: Record<string, string>,
): void => {
  const config = readConfig() ?? { accessToken: '' };
  config.integrations = config.integrations ?? {};
  config.integrations[providerName] = {
    cookies,
    lastUpdated: new Date().toISOString(),
  };
  writeConfig(config);
};

export const removeIntegrationAuth = (providerName: string): void => {
  const config = readConfig();
  if (config?.integrations?.[providerName]) {
    delete config.integrations[providerName];
    writeConfig(config);
  }
};
