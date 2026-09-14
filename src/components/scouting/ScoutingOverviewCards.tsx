import { Clock, Eye, User } from "lucide-react";

import type { StaffData } from "../../store/gameStore";
import { Card, CardBody } from "../ui";

interface ScoutingOverviewCardsProps {
  scouts: StaffData[];
  assignmentCount: number;
  availableScoutCount: number;
  totalCapacity: number;
  labels: {
    scouts: string;
    activeAssignments: string;
    freeSlots: string;
  };
}

export default function ScoutingOverviewCards({
  scouts,
  assignmentCount,
  availableScoutCount,
  totalCapacity,
  labels,
}: ScoutingOverviewCardsProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
      <Card>
        <CardBody>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-accent-500/10 flex items-center justify-center">
              <Eye className="w-5 h-5 text-accent-500" />
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">
                {labels.scouts}
              </p>
              <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">
                {scouts.length}
              </p>
            </div>
          </div>
        </CardBody>
      </Card>
      <Card>
        <CardBody>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary-500/10 flex items-center justify-center">
              <Clock className="w-5 h-5 text-primary-500" />
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">
                {labels.activeAssignments}
              </p>
              <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">
                {assignmentCount} / {totalCapacity}
              </p>
            </div>
          </div>
        </CardBody>
      </Card>
      <Card>
        <CardBody>
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-green-500/10 flex items-center justify-center">
              <User className="w-5 h-5 text-green-500" />
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400 font-heading uppercase tracking-wider">
                {labels.freeSlots}
              </p>
              <p className="text-xl font-heading font-bold text-gray-800 dark:text-gray-100">
                {availableScoutCount}
              </p>
            </div>
          </div>
        </CardBody>
      </Card>
    </div>
  );
}
