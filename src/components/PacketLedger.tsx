import type { Evidence } from "../api/types";
import { seconds } from "../lib/format";

/** A pause between consecutive packets at least this long gets its own row. */
const GAP_MS = 1000;

/**
 * Packet summary for exercises, readable without Wireshark. Long pauses between
 * packets are called out as their own rows because a timeout shows up as time passing.
 */
export function PacketLedger({ evidence }: { evidence: Evidence }) {
  const rows = evidence.packets.flatMap((packet, i) => {
    const previous = evidence.packets[i - 1];
    const gap = previous ? packet.offsetMs - previous.offsetMs : 0;
    const items = [];
    if (previous && gap >= GAP_MS) {
      items.push(
        <tr key={`gap-${packet.no}`} data-testid="gap-row">
          <td
            colSpan={6}
            className="border-y border-dashed border-moss-600 bg-ink-950 px-3 py-1.5 font-sans text-sm text-moss-300"
          >
            {`+${seconds(gap)} s passed between packet ${previous.no} and packet ${packet.no}`}
          </td>
        </tr>,
      );
    }
    items.push(
      <tr key={packet.no} className="border-b border-ink-700 last:border-b-0">
        <td className="px-3 py-2 text-mist">{packet.no}</td>
        <td className="px-3 py-2 tabular-nums">{seconds(packet.offsetMs)}</td>
        <td className="px-3 py-2 whitespace-nowrap">{packet.source}</td>
        <td className="px-3 py-2 whitespace-nowrap">{packet.destination}</td>
        <td className="px-3 py-2">{packet.protocol}</td>
        <td className="px-3 py-2">{packet.info}</td>
      </tr>,
    );
    return items;
  });

  const captionId = `ledger-${evidence.evidenceKey}`;
  return (
    <section aria-labelledby={captionId} className="mt-8">
      <h3 id={captionId} className="mb-2 text-lg font-bold">
        {evidence.title}
      </h3>
      <p className="mb-2 max-w-[68ch] text-sm text-mist">
        Fictional hosts. Times are seconds from the first packet.
      </p>
      <div
        role="region"
        aria-label={`${evidence.title}, packet table`}
        tabIndex={0}
        className="overflow-x-auto rounded border border-ink-700 bg-ink-800"
      >
        <table className="w-full border-collapse text-left font-mono text-[0.8rem]">
          <thead className="border-b border-ink-600 bg-ink-950 font-sans text-sm text-mist">
            <tr>
              <th scope="col" className="px-3 py-2 font-normal">No.</th>
              <th scope="col" className="px-3 py-2 font-normal">Time</th>
              <th scope="col" className="px-3 py-2 font-normal">Source</th>
              <th scope="col" className="px-3 py-2 font-normal">Destination</th>
              <th scope="col" className="px-3 py-2 font-normal">Protocol</th>
              <th scope="col" className="px-3 py-2 font-normal">Info</th>
            </tr>
          </thead>
          <tbody>{rows}</tbody>
        </table>
      </div>
    </section>
  );
}
