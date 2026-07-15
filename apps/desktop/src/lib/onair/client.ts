import { invokeDesktop } from "$lib/desktop/client";
import type {
  OnAirConnectionResult,
  OnAirCredentialProfileStatus,
} from "./types";

export function loadOnAirCredentialProfile(): Promise<OnAirCredentialProfileStatus> {
  return invokeDesktop<OnAirCredentialProfileStatus>(
    "onair_credential_profile_status",
  );
}

export function connectOnAir(
  companyId: string,
  apiKey: string,
  remember: boolean,
  connectOnStart: boolean,
): Promise<OnAirConnectionResult> {
  return invokeDesktop<OnAirConnectionResult>("connect_onair", {
    companyId,
    apiKey,
    remember,
    connectOnStart,
  });
}

export function connectRememberedOnAir(): Promise<OnAirConnectionResult> {
  return invokeDesktop<OnAirConnectionResult>("connect_remembered_onair");
}

export function autoConnectOnAirIfEnabled(): Promise<OnAirConnectionResult | null> {
  return invokeDesktop<OnAirConnectionResult | null>(
    "auto_connect_onair_if_enabled",
  );
}

export function forgetOnAirCredentials(): Promise<OnAirCredentialProfileStatus> {
  return invokeDesktop<OnAirCredentialProfileStatus>(
    "forget_onair_credentials",
  );
}
