
import useSWR from 'swr';
import { Blocks, BlocksMeta } from "../pkg/web";

export function useBlock(slot: string) {
  return useSWR(slot !== undefined ? ["block", slot] : null, (_, s) => Blocks.get("http://localhost:3000/data", BigInt(s)));
}

export function useBlocks(pageIndex: number, pageSize: number, sortId: string, sortDesc: boolean, meta: BlocksMeta) {
  return useSWR(
    () => {
      return {
        type: "blocks", pageIndex, pageSize, totalCount: meta.count, sortId, sortDesc
      }
    },
    key => Blocks.page(
      "http://localhost:3000/data",
      key.pageIndex,
      key.pageSize,
      key.totalCount,
      key.sortId,
      key.sortDesc
    )
  );
}

export function useCommittees(slot: string) {
  return useSWR(slot !== undefined ? ["committees", slot] : null, (_, s) => Blocks.committees("http://localhost:3000/data", BigInt(s)));
}

export function useVotes(slot: string) {
  return useSWR(slot !== undefined ? ["votes", slot] : null, (_, s) => Blocks.votes("http://localhost:3000/data", BigInt(s)));
}


export function useAttestations(slot: string) {
  return useSWR(slot !== undefined ? ["attestations", slot] : null, (_, s) => Blocks.attestations("http://localhost:3000/data", BigInt(s)));
}

export async function useMeta() {
  return useSWR(["blocks-meta"], (_, s) => Blocks.meta("http://localhost:3000/data"));
}