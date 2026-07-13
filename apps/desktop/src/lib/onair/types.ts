export type ConnectedCompany = {
  name: string;
  airline_code: string;
};

export type OnAirConnectionStatus = {
  connected: boolean;
  company: ConnectedCompany | null;
  credential_storage: "session_only";
};

export const disconnectedStatus: OnAirConnectionStatus = {
  connected: false,
  company: null,
  credential_storage: "session_only",
};
