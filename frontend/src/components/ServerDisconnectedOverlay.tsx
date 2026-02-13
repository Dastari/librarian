import { useEffect, useState } from "react";
import { Card, CardBody } from "@heroui/card";
import { Spinner } from "@heroui/spinner";
import { IconPlugConnectedX } from "@tabler/icons-react";
import {
  onWebSocketConnectionState,
  type WebSocketConnectionState,
} from "../lib/graphql/client";

export function ServerDisconnectedOverlay() {
  const [connectionState, setConnectionState] = useState<WebSocketConnectionState>({
    status: "idle",
    retryCount: 0,
    lastCloseCode: null,
    lastCloseReason: null,
    isAuthIssue: false,
  });

  useEffect(() => onWebSocketConnectionState(setConnectionState), []);

  if (connectionState.status !== "disconnected") return null;

  return (
    <div className="fixed inset-0 z-[200] bg-black/70 backdrop-blur-sm flex items-center justify-center p-6">
      <Card className="w-full max-w-lg border border-danger/40 bg-content1/90">
        <CardBody className="p-6 sm:p-8 text-center space-y-4">
          <IconPlugConnectedX size={48} className="mx-auto text-danger-400" />
          <h2 className="text-2xl font-semibold">Server disconnected</h2>
          <p className="text-default-500">
            Real-time connection to the server was lost.
          </p>
          <div className="flex items-center justify-center gap-2 text-sm text-default-400">
            <Spinner size="sm" />
            <span>
              Retrying connection{connectionState.retryCount > 0
                ? ` (attempt ${connectionState.retryCount})`
                : ""}
              ...
            </span>
          </div>
        </CardBody>
      </Card>
    </div>
  );
}
