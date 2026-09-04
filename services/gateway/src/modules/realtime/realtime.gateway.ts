import { WebSocketGateway } from "@nestjs/websockets";

/**
 * §11.2: WS /ws/stream, topics: ticks:{sym}, bars:{sym}:{tf}, signals,
 * positions, pnl, agent_status. Binary MessagePack framing and per-topic
 * RBAC (§11.3) are Phase 4 scope — this fixes the gateway's existence and
 * path so the dashboard's WS client has a stable target to connect to.
 */
@WebSocketGateway({ path: "/ws/stream" })
export class RealtimeGateway {}
