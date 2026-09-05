/**
 * Largest-Triangle-Three-Buckets downsampling (§11.2's "downsampled" bars
 * API, §12.2's `LodManager`). Generic over the point type so the same
 * implementation serves both OHLC bars (keyed on close) on the server side
 * and the client-side zoom-level downsampling `LodManager` will do — a
 * single source of truth for both, rather than two hand-tuned copies.
 *
 * Preserves the actual data point picked to represent each bucket (not an
 * aggregate/average of the bucket) so an OHLC bar downsampled this way is
 * still a real bar that really occurred, just a sparser selection of them —
 * the property a candlestick chart needs to stay honest at low zoom.
 */
export function lttbDownsample<T>(
  data: readonly T[],
  threshold: number,
  timeOf: (point: T) => number,
  valueOf: (point: T) => number,
): T[] {
  if (threshold <= 0 || data.length <= threshold || data.length <= 2) {
    return data.slice();
  }

  const sampled: T[] = [data[0]];
  // Buckets exclude the fixed first/last points, so there are threshold-2 of them.
  const bucketSize = (data.length - 2) / (threshold - 2);
  let selectedIndex = 0;

  for (let bucket = 0; bucket < threshold - 2; bucket++) {
    // The next bucket's average point anchors the triangle whose area we're maximizing.
    const nextBucketStart = Math.floor((bucket + 1) * bucketSize) + 1;
    const nextBucketEnd = Math.min(Math.floor((bucket + 2) * bucketSize) + 1, data.length);
    let avgX = 0;
    let avgY = 0;
    const nextCount = nextBucketEnd - nextBucketStart;
    for (let i = nextBucketStart; i < nextBucketEnd; i++) {
      avgX += timeOf(data[i]);
      avgY += valueOf(data[i]);
    }
    avgX /= nextCount;
    avgY /= nextCount;

    // This bucket's own candidate range, searched for the point forming the
    // largest triangle with the previously selected point and that average.
    const rangeStart = Math.floor(bucket * bucketSize) + 1;
    const rangeEnd = Math.floor((bucket + 1) * bucketSize) + 1;

    const pointA = data[selectedIndex];
    let maxArea = -1;
    let nextSelectedIndex = rangeStart;
    for (let i = rangeStart; i < rangeEnd; i++) {
      const area = Math.abs(
        (timeOf(pointA) - avgX) * (valueOf(data[i]) - valueOf(pointA)) -
          (timeOf(pointA) - timeOf(data[i])) * (avgY - valueOf(pointA)),
      );
      if (area > maxArea) {
        maxArea = area;
        nextSelectedIndex = i;
      }
    }
    sampled.push(data[nextSelectedIndex]);
    selectedIndex = nextSelectedIndex;
  }

  sampled.push(data[data.length - 1]);
  return sampled;
}
