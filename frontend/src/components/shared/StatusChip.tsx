import { Chip } from '@heroui/chip'
import { IconCheck, IconAlertTriangle, IconX } from '@tabler/icons-react'

export type StatusType = 'monitored' | 'unmonitored' | 'active' | 'disabled' | 'error' | 'warning' | 'inheriting'

interface StatusChipProps {
  status: StatusType
  size?: 'sm' | 'md' | 'lg'
  /** Custom label override */
  label?: string
  /** Show icon prefix */
  showIcon?: boolean
}

const STATUS_CONFIG: Record<StatusType, {
  color: 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'
  label: string
  icon?: React.ComponentType<{ size: number; className?: string }>
}> = {
  monitored: {
    color: 'success',
    label: 'Monitored',
    icon: IconCheck,
  },
  unmonitored: {
    color: 'default',
    label: 'Unmonitored',
  },
  active: {
    color: 'success',
    label: 'Active',
    icon: IconCheck,
  },
  disabled: {
    color: 'default',
    label: 'Disabled',
  },
  error: {
    color: 'danger',
    label: 'Error',
    icon: IconX,
  },
  warning: {
    color: 'warning',
    label: 'Warning',
    icon: IconAlertTriangle,
  },
  inheriting: {
    color: 'default',
    label: 'Inheriting from library',
  },
}

/**
 * A reusable status chip for consistent status display across the app.
 * Used for monitored/unmonitored, active/disabled, error states, etc.
 */
export function StatusChip({ status, size = 'sm', label, showIcon = false }: StatusChipProps) {
  const config = STATUS_CONFIG[status]
  const Icon = config.icon
  
  return (
    <Chip
      size={size}
      color={config.color}
      variant="flat"
      startContent={showIcon && Icon ? <Icon size={12} /> : undefined}
    >
      {label || config.label}
    </Chip>
  )
}
