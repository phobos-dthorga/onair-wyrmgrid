import type { TranslationKey } from "$lib/i18n/catalog";

export type DispatchFindingMessageKeys = {
  title: TranslationKey;
  explanation: TranslationKey;
};

export const dispatchFindingMessageKeys: Readonly<
  Record<string, DispatchFindingMessageKeys>
> = {
  "dispatch-finding-aircraft-vocabularies": keys(
    "dispatch-finding-aircraft-vocabularies-title",
    "dispatch-finding-aircraft-vocabularies-explanation",
  ),
  "dispatch-finding-airframe-unmatched": keys(
    "dispatch-finding-airframe-unmatched-title",
    "dispatch-finding-airframe-unmatched-explanation",
  ),
  "dispatch-finding-deadline-unavailable": keys(
    "dispatch-finding-deadline-unavailable-title",
    "dispatch-finding-deadline-unavailable-explanation",
  ),
  "dispatch-finding-job-deadline-match": keys(
    "dispatch-finding-job-deadline-match-title",
    "dispatch-finding-job-deadline-match-explanation",
  ),
  "dispatch-finding-job-deadline-missed": keys(
    "dispatch-finding-job-deadline-missed-title",
    "dispatch-finding-job-deadline-missed-explanation",
  ),
  "dispatch-finding-job-deadline-unavailable": keys(
    "dispatch-finding-job-deadline-unavailable-title",
    "dispatch-finding-job-deadline-unavailable-explanation",
  ),
  "dispatch-finding-job-payload-difference": keys(
    "dispatch-finding-job-payload-difference-title",
    "dispatch-finding-job-payload-difference-explanation",
  ),
  "dispatch-finding-job-payload-match": keys(
    "dispatch-finding-job-payload-match-title",
    "dispatch-finding-job-payload-match-explanation",
  ),
  "dispatch-finding-job-payload-unavailable": keys(
    "dispatch-finding-job-payload-unavailable-title",
    "dispatch-finding-job-payload-unavailable-explanation",
  ),
  "dispatch-finding-job-route-difference": keys(
    "dispatch-finding-job-route-difference-title",
    "dispatch-finding-job-route-difference-explanation",
  ),
  "dispatch-finding-job-route-match": keys(
    "dispatch-finding-job-route-match-title",
    "dispatch-finding-job-route-match-explanation",
  ),
  "dispatch-finding-job-route-unavailable": keys(
    "dispatch-finding-job-route-unavailable-title",
    "dispatch-finding-job-route-unavailable-explanation",
  ),
  "dispatch-finding-job-unselected": keys(
    "dispatch-finding-job-unselected-title",
    "dispatch-finding-job-unselected-explanation",
  ),
  "dispatch-finding-model-candidate": keys(
    "dispatch-finding-model-candidate-title",
    "dispatch-finding-model-candidate-explanation",
  ),
  "dispatch-finding-model-difference": keys(
    "dispatch-finding-model-difference-title",
    "dispatch-finding-model-difference-explanation",
  ),
  "dispatch-finding-model-match": keys(
    "dispatch-finding-model-match-title",
    "dispatch-finding-model-match-explanation",
  ),
  "dispatch-finding-model-unavailable": keys(
    "dispatch-finding-model-unavailable-title",
    "dispatch-finding-model-unavailable-explanation",
  ),
  "dispatch-finding-onair-fleet-unavailable": keys(
    "dispatch-finding-onair-fleet-unavailable-title",
    "dispatch-finding-onair-fleet-unavailable-explanation",
  ),
  "dispatch-finding-payload-unavailable": keys(
    "dispatch-finding-payload-unavailable-title",
    "dispatch-finding-payload-unavailable-explanation",
  ),
  "dispatch-finding-plan-aircraft-missing": keys(
    "dispatch-finding-plan-aircraft-missing-title",
    "dispatch-finding-plan-aircraft-missing-explanation",
  ),
  "dispatch-finding-position-difference": keys(
    "dispatch-finding-position-difference-title",
    "dispatch-finding-position-difference-explanation",
  ),
  "dispatch-finding-position-match": keys(
    "dispatch-finding-position-match-title",
    "dispatch-finding-position-match-explanation",
  ),
  "dispatch-finding-position-unavailable": keys(
    "dispatch-finding-position-unavailable-title",
    "dispatch-finding-position-unavailable-explanation",
  ),
  "dispatch-finding-registration-ambiguous": keys(
    "dispatch-finding-registration-ambiguous-title",
    "dispatch-finding-registration-ambiguous-explanation",
  ),
  "dispatch-finding-registration-match": keys(
    "dispatch-finding-registration-match-title",
    "dispatch-finding-registration-match-explanation",
  ),
  "dispatch-finding-registration-not-found": keys(
    "dispatch-finding-registration-not-found-title",
    "dispatch-finding-registration-not-found-explanation",
  ),
};

function keys(
  title: TranslationKey,
  explanation: TranslationKey,
): DispatchFindingMessageKeys {
  return { title, explanation };
}
