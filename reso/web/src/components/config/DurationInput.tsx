import { HStack, Input, NativeSelect } from '@chakra-ui/react';
import { useCallback, useMemo } from 'react';

const UNITS = [
	{ label: 'minutes', value: 60 },
	{ label: 'hours', value: 3600 },
	{ label: 'days', value: 86400 },
] as const;

type Unit = (typeof UNITS)[number]['value'];

function decompose(totalSeconds: number): { amount: number; unit: Unit } {
	// Pick the largest unit that divides evenly, falling back to minutes
	for (let i = UNITS.length - 1; i >= 0; i--) {
		const u = UNITS[i];
		if (totalSeconds >= u.value && totalSeconds % u.value === 0) {
			return { amount: totalSeconds / u.value, unit: u.value };
		}
	}
	// Doesn't divide evenly into any unit — show as minutes (rounded)
	return { amount: Math.round(totalSeconds / 60), unit: 60 };
}

interface Props {
	value: number;
	onChange: (seconds: number) => void;
	min?: number;
}

export function DurationInput({ value, onChange, min = 60 }: Props) {
	const { amount, unit } = useMemo(() => decompose(value), [value]);

	const handleAmountChange = useCallback(
		(e: React.ChangeEvent<HTMLInputElement>) => {
			const n = Number.parseInt(e.target.value, 10);
			if (!Number.isNaN(n) && n > 0) {
				onChange(Math.max(n * unit, min));
			}
		},
		[unit, min, onChange],
	);

	const handleUnitChange = useCallback(
		(e: React.ChangeEvent<HTMLSelectElement>) => {
			const newUnit = Number.parseInt(e.target.value, 10) as Unit;
			onChange(Math.max(amount * newUnit, min));
		},
		[amount, min, onChange],
	);

	return (
		<HStack gap='2'>
			<Input
				type='number'
				min={1}
				step={1}
				value={amount}
				onChange={handleAmountChange}
				flex='1'
			/>
			<NativeSelect.Root w='auto' minW='110px'>
				<NativeSelect.Field
					bg='bg.input'
					borderColor='border.input'
					value={unit}
					onChange={handleUnitChange}
				>
					{UNITS.map((u) => (
						<option key={u.value} value={u.value}>
							{u.label}
						</option>
					))}
				</NativeSelect.Field>
				<NativeSelect.Indicator />
			</NativeSelect.Root>
		</HStack>
	);
}
