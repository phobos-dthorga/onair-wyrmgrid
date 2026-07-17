# Flight operations in Dispatch

WyrmGrid can preserve an accepted plan and selected read-only OnAir job as a
local flight operation. This is a planning record inside WyrmGrid. It does not
file a flight plan, accept or alter an OnAir job, control the simulator, or
certify a flight for real-world operation.

## Begin an operation

1. Open **Jobs** and select a pending job if one is relevant.
2. Open **Dispatch** and import the latest SimBrief plan.
3. Review the plan, job comparison, and flight-operation journey.
4. Select **Begin flight operation**.

WyrmGrid creates revision 1 in its encrypted local database. A job is optional,
but a validated plan is required. When a job is attached, the Manifest section
shows each supplied leg, aggregate passenger count, and freight weight. A field
that OnAir did not report says **Unavailable**; WyrmGrid does not estimate it.
The job is internally bound to its originating OnAir company so changing
connections cannot silently reattribute old evidence.

## Review a change

Importing another plan, selecting another job, or receiving changed facts for
the selected job does not rewrite the accepted operation. Dispatch displays a
change notice and offers **Create reviewed revision**. Use it only after you
have reviewed the new combination. The operation keeps its identity, advances
its revision number, and retains the older revision locally.

The journey rail is not a forced wizard. It links to available workspaces and
uses host-derived states:

- **Ready** means the stated evidence for that stage is present;
- **Available** means the workspace can be used but is not accepted or complete;
- **Needs attention** means retained evidence contains a visible gap;
- **Stale** means newer context differs from the accepted evidence;
- **Unavailable** means a required source or snapshot is absent; and
- **Not started** means the operation has not yet reached that stage.

These labels are planning assistance, not airworthiness, regulatory, staffing,
or commercial approval.

## Restart, backup, and current limitations

The accepted operation survives restart and remains visible in Dispatch even
before a new session plan is imported. Re-import a current plan when you want
to compare or revise it. Encrypted portable backups include the operation and
all retained revisions because they copy the complete WyrmGrid database.

This foundation records only sanitized plan evidence, an optional selected job,
and job-derived aggregate manifest facts. It does not yet assign aircraft or
staff, identify individual passengers or consignments, bind a Bridge recording,
open operation history, or create a Hoard debrief association. Those stages
remain visible as honest availability states while their reviewed core models
are implemented.

## Suggested local test

1. Select a pending OnAir job and import a SimBrief plan.
2. Begin the operation and confirm the route, revision number, and manifest
   match the supplied sources.
3. Confirm any missing passenger or freight field is shown as unavailable.
4. Select a different job or import a different plan. Confirm revision 1 is
   unchanged and a reviewed-revision action appears.
5. Create the revision and confirm the operation identity is retained while the
   revision number increments.
6. Restart WyrmGrid and open Dispatch before importing a plan. Confirm the
   accepted revision is still visible.
