# dailyschedule-rust

This rust library is intended to run a daily schedule to control devices
for home automation purposes. It supports randomness and custom hooks for
dynamic scheduling behavior. It is capable to handle DST changes, even
when the host system don't have explicit support for it or is configured
to a different timezone.

This library is only capable of calculating and tracking a schedule, but
has no functionality to run the schedule automatically.

I doubt this library is capable to be executed in a Windows environment.
