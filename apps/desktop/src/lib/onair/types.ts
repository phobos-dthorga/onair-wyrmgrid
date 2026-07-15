export type ConnectedCompany = {
  name: string;
  airline_code: string;
};

export type OnAirConnectionStatus = {
  connected: boolean;
  company: ConnectedCompany | null;
  credential_storage: "session_only";
};

export type OnAirCredentialProfileStatus = {
  remembered: boolean;
  company_id?: string;
  connect_on_start: boolean;
  secret_available: boolean;
  credential_store_available: boolean;
};

export type OnAirConnectionResult = {
  connection: OnAirConnectionStatus;
  profile: OnAirCredentialProfileStatus;
};

export const emptyCredentialProfile: OnAirCredentialProfileStatus = {
  remembered: false,
  connect_on_start: false,
  secret_available: false,
  credential_store_available: true,
};

export const disconnectedStatus: OnAirConnectionStatus = {
  connected: false,
  company: null,
  credential_storage: "session_only",
};
